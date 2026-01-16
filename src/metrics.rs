use crate::{
    config::{IngestorConfig, MetricsConfig},
    metrics_core::{
        MetricDefinitions, MetricFuncResult, MetricFutures, MetricLabels, metric_spawner,
        prometheus::{TimeSeries, WriteRequest},
        sample_now,
    },
    simple_metric, system_metric,
};

use prost::Message;
use snap::raw::Encoder;
use std::{collections::HashMap, process::exit, sync::Arc, time::Duration};
use sysinfo::{CpuRefreshKind, System};
use tokio::{
    sync::{
        Mutex,
        mpsc::{self, UnboundedSender},
    },
    time::sleep,
};

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

//
// METRIC FUNCTION DEFINITIONS
//
// Metrics must return a numerical value for the metric (required by prometheus) as well as an optional extra set of labels to append
//

/// Runs 'bec --version --json' and returns the result, hopefully the most recent pip versions of the BEC libraries.
/// If the command fails, returns a placeholder.
fn get_versions() -> MetricLabels {
    use std::process::{Command, Stdio};

    let bec_versions_raw = match Command::new("bec")
        .arg("--version")
        .arg("--json")
        .stdout(Stdio::piped())
        .output()
    {
        Ok(bec_versions_raw) => bec_versions_raw,
        Err(e) => {
            println!("ERROR: executing 'bec --version --json' failed: {e}");
            return HashMap::from([("versions".into(), "failed to run command".into())]);
        }
    };
    let Ok(parsed) = serde_json::from_slice(&bec_versions_raw.stdout) else {
        println!(
            "ERROR: decoding output from 'bec --version --json' failed: received '{}'",
            String::from_utf8(bec_versions_raw.stdout).unwrap_or("<invalid utf8>".into())
        );
        return HashMap::from([("versions".into(), "failed to run command".into())]);
    };
    parsed
}

fn deployment() -> MetricFuncResult {
    let mut extra_labels: MetricLabels = HashMap::from([]);
    extra_labels.extend(get_versions());
    (sample_now(1.into()), Some(extra_labels))
}

// System info metrics
fn cpu_usage_percent(system: &mut System) -> MetricFuncResult {
    system.refresh_cpu_specifics(CpuRefreshKind::nothing().with_cpu_usage());
    (sample_now(system.global_cpu_usage().into()), None)
}
fn ram_usage_bytes(system: &mut System) -> MetricFuncResult {
    (sample_now(system.used_memory() as f64), None)
}
fn ram_available_bytes(system: &mut System) -> MetricFuncResult {
    (sample_now(system.available_memory() as f64), None)
}

/// Defines the list of all metrics to run
fn metric_definitions(config: &IngestorConfig) -> MetricDefinitions {
    Arc::new(HashMap::from([
        // Custom/simple metrics
        simple_metric!(deployment, [("beamline", &config.loki.beamline_name)]),
        // System info metrics
        system_metric!(
            cpu_usage_percent,
            [("beamline", &config.loki.beamline_name)]
        ),
        system_metric!(ram_usage_bytes, [("beamline", &config.loki.beamline_name)]),
        system_metric!(
            ram_available_bytes,
            [("beamline", &config.loki.beamline_name)]
        ),
    ]))
}

//
// ROUTINES TO PROCESS METRICS
//

/// Checks the running futures for any that have failed and attempts to restart them, once per minute.
async fn watchdog_loop(
    futs: MetricFutures,
    metrics: MetricDefinitions,
    tx: UnboundedSender<TimeSeries>,
    config: &'static IngestorConfig,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    let mut spawner = metric_spawner(tx.clone(), config);

    loop {
        interval.tick().await;

        let finished: Vec<String> = {
            futs.lock()
                .await
                .iter()
                .filter(|(_, fut)| fut.is_finished())
                .map(|(name, _)| name.clone())
                .collect()
        };

        if !finished.is_empty() {
            println!(
                "ERROR: The following metric coroutines have crashed, restarting them: {finished:?}",
            );
            for name in finished {
                if let Some((func, labels)) = metrics.get(&name) {
                    let (_, handle) = spawner((&name, &(func.clone(), labels.clone())));
                    futs.lock().await.insert(name, handle);
                }
            }
        }
    }
}

/// Consumes any metrics passed into the channel and encodes them in a Prometheus WriteRequest
async fn consumer_loop(rx: &mut mpsc::UnboundedReceiver<TimeSeries>, config: MetricsConfig) {
    let chunk_size = 100;
    let mut buffer: Vec<TimeSeries> = Vec::with_capacity(chunk_size);
    let mut proto_encoded_buffer: Vec<u8> = Vec::new();
    let mut snap_encoder = Encoder::new();
    let mut retries: u8 = 0;
    let client = reqwest::Client::new();

    loop {
        let open = rx.recv_many(&mut buffer, chunk_size).await;
        if open == 0 {
            break;
        }
        let write_request = WriteRequest {
            timeseries: buffer.clone(),
        };
        proto_encoded_buffer.reserve(write_request.encoded_len());
        let Ok(()) = write_request.encode(&mut proto_encoded_buffer) else {
            println!("ERROR: encountered an error in protobuf encoding. Exiting.");
            break;
        };
        let Ok(compressed) = snap_encoder.compress_vec(&proto_encoded_buffer) else {
            println!("ERROR: encountered an error in snappy compression. Exiting.");
            break;
        };

        match client
            .post(&config.url)
            .header("Content-Type", "application/x-protobuf")
            .header("Content-Encoding", "snappy")
            .header("User-Agent", format!("bec_log_ingestor v{APP_VERSION}"))
            .basic_auth(&config.auth.username, Some(&config.auth.password))
            .body(compressed)
            .send()
            .await
        {
            Ok(res) => {
                retries = 0;
                println!("Sent {open} metrics to Mimir.");
                if !res.status().is_success() {
                    let text = res
                        .text()
                        .await
                        .unwrap_or("[Unable to decode response text!]".into());
                    println!("Received error response: {text} ");
                }
            }
            Err(res) => {
                println!("ERROR: {res:?}");
                if retries == 3 {
                    println!("Maximum retry attempts exceeded, exiting.");
                    exit(0x45); // Service unavailable
                }
                println!("Retrying in 5s.");
                sleep(Duration::from_secs(5)).await;
                retries += 1;
            }
        };
        buffer.clear();
        proto_encoded_buffer.clear();
    }
    println!("Producer dropped, consumer exiting");
}

/// Main routine to start the metrics service
pub async fn metrics_loop(config: &'static IngestorConfig) {
    let (tx, mut rx) = mpsc::unbounded_channel::<TimeSeries>();

    let metrics = metric_definitions(&config);
    let spawner = metric_spawner(tx.clone(), config);
    let futs: MetricFutures = Arc::new(Mutex::new(metrics.iter().map(spawner).collect()));

    let watchdog = {
        let futs = Arc::clone(&futs);
        tokio::spawn(watchdog_loop(futs, metrics, tx.clone(), config))
    };

    consumer_loop(&mut rx, config.metrics.clone()).await;
    watchdog.abort();
    let _ = watchdog.await;
    rx.close();
    // with rx closed, all futures should exit on their next iteration, but some could be at too long
    // of an interval to wait for
    for handle in futs.lock().await.values() {
        handle.abort(); // abort each task
    }
}
