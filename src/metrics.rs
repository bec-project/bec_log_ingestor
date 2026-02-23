use crate::{
    STOP_METRICS,
    config::{IngestorConfig, MetricsConfig},
    metrics_core::{
        MetricDefinition, MetricDefinitions, MetricError, MetricFutures, MetricLabels,
        PinMetricResultFut, RedisMetricFunc, SysMetricFunc, metric_spawner,
        prometheus::{TimeSeries, WriteRequest},
        sample_now, static_metric_def, sync_metric,
    },
    status_message::StatusMessagePack,
};

use prost::Message;
use redis::{AsyncCommands, aio::MultiplexedConnection};
use snap::raw::Encoder;
use std::{
    collections::HashMap,
    process::exit,
    sync::{Arc, atomic::Ordering},
    time::Duration,
};
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, System};
use tokio::{
    sync::{
        Mutex,
        mpsc::{self, UnboundedSender},
    },
    time::{Interval, sleep},
};

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

//
// METRIC FUNCTION DEFINITIONS
//
// Metrics must return a numerical value for the metric (required by prometheus) as well as an optional extra set of labels to append
//

fn deployment(redis: &mut MultiplexedConnection) -> PinMetricResultFut<'_> {
    Box::pin(async move {
        let key = "user/services/status/DeviceServer";
        let val: Vec<u8> = redis.get(&key).await.map_err(|e| {
            MetricError::Retryable(e.detail().unwrap_or("Unspecified redis error").into())
        })?;
        if val.is_empty() {
            return Err(MetricError::Retryable(format!(
                "No status update found at {key}"
            )));
        }
        let status_update: StatusMessagePack =
            rmp_serde::from_slice(val.as_slice()).map_err(|e| {
                MetricError::Retryable(format!(
                    "Failed to parse status message from redis: {}",
                    e.to_string()
                ))
            })?;
        let mut extra_labels: MetricLabels = HashMap::from([]);
        let service_info = status_update.bec_codec.data.info;
        match service_info {
            crate::status_message::Info::ServiceInfo(info) => {
                let versions = info.bec_codec.data.versions.unwrap();
                extra_labels.insert("bec_lib".into(), versions.bec_lib);
                extra_labels.insert("bec_ipython_client".into(), versions.bec_ipython_client);
                extra_labels.insert("bec_server".into(), versions.bec_server);
                extra_labels.insert("bec_widgets".into(), versions.bec_widgets);
            }
            _ => {
                return Err(MetricError::Fatal(format!(
                    "Status update {service_info:?} contained malformed ServiceInfo"
                )));
            }
        }
        // extra_labels.extend(get_versions());
        Ok((sample_now(1.into()), Some(extra_labels)))
    })
}

// System info metrics
fn cpu_usage_pc(system: &mut System) -> PinMetricResultFut<'_> {
    system.refresh_cpu_specifics(CpuRefreshKind::nothing().with_cpu_usage());
    sync_metric(Ok((sample_now(system.global_cpu_usage() as f64), None)))
}
fn ram_used_bytes(system: &mut System) -> PinMetricResultFut<'_> {
    system.refresh_memory_specifics(MemoryRefreshKind::nothing().with_ram());
    sync_metric(Ok((sample_now(system.used_memory() as f64), None)))
}
fn ram_avail_bytes(system: &mut System) -> PinMetricResultFut<'_> {
    system.refresh_memory_specifics(MemoryRefreshKind::nothing().with_ram());
    sync_metric(Ok((sample_now(system.available_memory() as f64), None)))
}

/// Defines the list of all metrics to run
fn metric_definitions(config: &'static IngestorConfig) -> MetricDefinitions {
    // set up the statically defined metrics first
    let mut metrics = HashMap::from([
        static_metric_def(
            Arc::new(deployment) as RedisMetricFunc,
            "deployment",
            (&config, Some(60), None),
        ),
        // System info metrics
        static_metric_def(
            Arc::new(cpu_usage_pc) as SysMetricFunc,
            "cpu_usage_pc",
            (&config, None, None),
        ),
        static_metric_def(
            Arc::new(ram_used_bytes) as SysMetricFunc,
            "ram_used_bytes",
            (&config, None, None),
        ),
        static_metric_def(
            Arc::new(ram_avail_bytes) as SysMetricFunc,
            "ram_avail_bytes",
            (&config, None, None),
        ),
    ]);
    // load the dynamically defined metrics from the config
    metrics.extend(
        config
            .metrics
            .dynamic
            .iter()
            .map(|(n, dm)| (n.to_owned(), MetricDefinition::Dynamic(dm.clone()))),
    );
    Arc::new(metrics)
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
    redis: MultiplexedConnection,
) {
    let mut interval: Interval = (&config.metrics.watchdog_interval).into();
    let mut spawner = metric_spawner(tx.clone(), config.clone(), redis);
    interval.tick().await;

    loop {
        if STOP_METRICS.load(Ordering::Relaxed) {
            break;
        }
        interval.tick().await;
        println!("Watchdog checking {:?}", futs.lock().await.keys());
        dbg!(tx.strong_count());
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
                if let Some(metric_def) = metrics.get(&name) {
                    match metric_def {
                        crate::metrics_core::MetricDefinition::Static(_) => {
                            let (_, handle) = spawner((&name, metric_def));
                            futs.lock().await.insert(name, handle);
                        }
                        crate::metrics_core::MetricDefinition::Dynamic(_) => todo!(),
                    }
                }
            }
        }
        println!("Watchdog done.");
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
    let mut interval: Interval = (&config.publish_interval).into();

    loop {
        if STOP_METRICS.load(Ordering::Relaxed) {
            break;
        }
        interval.tick().await;
        println!("DEBUG: publishing metrics to Mimir");
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

    let client =
        redis::Client::open(config.redis.url.full_url()).expect("Failed to connect to redis!");
    let redis = client
        .get_multiplexed_async_connection()
        .await
        .expect("Failed to connect to redis!");
    let metrics = metric_definitions(&config);
    let spawner = metric_spawner(tx.clone(), config.clone(), redis.clone());

    let futs: MetricFutures = Arc::new(Mutex::new(metrics.iter().map(spawner).collect()));

    let watchdog = {
        let futs = Arc::clone(&futs);
        tokio::spawn(watchdog_loop(
            futs,
            metrics,
            tx.clone(),
            config,
            redis.clone(),
        ))
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use tokio::spawn;

    use crate::{
        config::{
            BasicAuth, IngestorConfig, LokiConfig, MetricInterval, MetricsConfig, RedisConfig,
            UrlPort,
        },
        metrics::{consumer_loop, metric_definitions},
        metrics_core::{
            MetricDefinition,
            prometheus::{Label, Sample, TimeSeries},
        },
    };

    fn test_config(
        url: String,
        intervals: Option<HashMap<String, MetricInterval>>,
    ) -> MetricsConfig {
        MetricsConfig {
            user_config_path: None,
            auth: BasicAuth {
                username: "user".into(),
                password: "pass".into(),
            },
            url: url,
            intervals: intervals.unwrap_or(HashMap::new()),
            dynamic: HashMap::new(),
            watchdog_interval: MetricInterval::Daily(1),
            publish_interval: MetricInterval::Millis(1),
        }
    }

    #[test]
    fn test_definitions() {
        let intervals: HashMap<String, MetricInterval> = HashMap::from([]);
        let config = IngestorConfig {
            redis: RedisConfig {
                url: UrlPort {
                    url: "".into(),
                    port: 12345,
                },
                chunk_size: 5,
                blocktime_millis: 5,
                consumer_group: "".into(),
                consumer_id: "".into(),
            },
            loki: LokiConfig {
                url: "".into(),
                auth: BasicAuth {
                    username: "user".into(),
                    password: "pass".into(),
                },
                chunk_size: 5,
                beamline_name: "test".into(),
            },
            metrics: test_config("".into(), Some(intervals.clone())),
            enable_metrics: false,
            enable_logging: false,
        };
        let defs = metric_definitions(Box::leak(Box::new(config)));
        if let MetricDefinition::Static(def) = defs.get("cpu_usage_pc").unwrap() {
            assert_eq!(def.2, None);
        } else {
            panic!("wrong type!")
        }
        if let MetricDefinition::Static(def) = defs.get("deployment").unwrap() {
            assert_eq!(def.2, Some(60));
        } else {
            panic!("wrong type!")
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_consumer_loop() {
        let mut server = mockito::Server::new_async().await;
        let url = server.url();
        let config = test_config(url, None);
        let mock = server
            .mock("POST", "/")
            .with_status(201)
            .with_header("Content-Type", "application/x-protobuf")
            .with_header("Content-Encoding", "snappy")
            .match_request(|req| req.body().unwrap().len() == 33)
            .create();

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<TimeSeries>();
        let handle = spawn(consumer_loop(Box::leak(Box::new(rx)), config));

        let _ = tx.send(TimeSeries {
            labels: Vec::from([Label {
                name: "__name__".into(),
                value: "test".into(),
            }]),
            samples: Vec::from([Sample {
                value: 1.0,
                timestamp: 0,
            }]),
        });
        drop(tx);
        let _ = handle.await;

        mock.assert();
    }
}
