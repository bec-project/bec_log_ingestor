use crate::{
    config::{IngestorConfig, MetricsConfig, RedisConfig},
    metrics::prometheus::Label,
    redis_logs::create_redis_conn,
};

use prost::Message;
use redis::Connection;
use snap::raw::Encoder;
use std::collections::HashMap;
use sysinfo::{CpuRefreshKind, System};
use tokio::{
    spawn,
    sync::mpsc::{self, UnboundedSender},
    task::JoinHandle,
    time,
};

pub mod prometheus {
    include!(concat!(env!("OUT_DIR"), "/prometheus.rs"));
}
use prometheus::{Sample, TimeSeries, WriteRequest};

type SimpleMetricFunc = Box<dyn Fn() -> Sample + Send + Sync>;
type SysinfoMetricFunc = Box<dyn Fn(&mut System) -> Sample + Send + Sync>;
type RedisMetricFunc = Box<dyn Fn(&mut Connection) -> Sample + Send + Sync>;
type MetricLabels = HashMap<String, String>;

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

enum MetricFunc {
    Simple(SimpleMetricFunc),
    Redis(RedisMetricFunc),
    System(SysinfoMetricFunc),
}

fn get_timestamp() -> i64 {
    let ts = chrono::Utc::now().timestamp_millis();
    dbg!(ts)
}

fn sample_now(value: f64) -> Sample {
    println!("sample_now called with {value}");
    Sample {
        timestamp: get_timestamp(),
        value,
    }
}
fn labels_from_hashmap(labels: &MetricLabels) -> Vec<Label> {
    labels
        .iter()
        .map(|(k, v)| Label {
            name: k.into(),
            value: v.into(),
        })
        .collect()
}
macro_rules! transmit_metric_loop {
    ($func:expr, ($($args:expr),*), $interval:expr, $tx:expr, $labels:expr) => {
        loop {
            $interval.tick().await;
            println!("loop iterated");
            let sample = vec![dbg!($func($($args),*))];
            if $tx
                .send(dbg!(TimeSeries {
                    labels: labels_from_hashmap(&$labels),
                    samples: sample,
                }))
                .is_err()
            {
                break;
            }
        }
    };
}

/// Create a future for a given metric function.
async fn metric_future(
    metric_func: MetricFunc,
    labels: HashMap<String, String>,
    mut interval: time::Interval,
    redis_config: RedisConfig,
    tx: UnboundedSender<TimeSeries>,
) {
    match metric_func {
        MetricFunc::Simple(ref func) => transmit_metric_loop!(func, (), interval, tx, labels),
        MetricFunc::Redis(ref func) => {
            let mut redis = create_redis_conn(&redis_config.url.full_url())
                .expect("Could not connect to Redis!");
            transmit_metric_loop!(func, (&mut redis), interval, tx, labels)
        }
        MetricFunc::System(ref func) => {
            let mut system = System::new_all();
            transmit_metric_loop!(func, (&mut system), interval, tx, labels)
        }
    };
}

// Macros create a hashmap entry (name, (func, labels)) from a conforming function
// These can be simplified to a nested macro when the feature is stabilized: https://github.com/rust-lang/rust/issues/83527
macro_rules! simple_metric {
    ($func:expr, [ $(($label_k:expr, $label_v:expr)),*]) => {
        (
            stringify!($func).to_string(),
            (
                $crate::metrics::Simple(Box::new($func) as $crate::metrics::SimpleMetricFunc),
                std::collections::HashMap::from([$(($label_k.into(), $label_v.into())),*]),
            )
        )
    };
}
macro_rules! system_metric {
    ($func:expr, [ $(($label_k:expr, $label_v:expr)),*]) => {
        (
            stringify!($func).to_string(),
            (
                $crate::metrics::MetricFunc::System(Box::new($func) as $crate::metrics::SysinfoMetricFunc),
                std::collections::HashMap::from([$(($label_k.into(), $label_v.into())),*]),
            ),
        )
    };
}
macro_rules! redis_metric {
    ($func:expr, [ $(($label_k:expr, $label_v:expr)),*]) => {
        (
            stringify!($func).to_string(),
            (
                $crate::metrics::MetricFunc::Redis(Box::new($func) as $crate::metrics::RedisMetricFunc),
                std::collections::HashMap::from([$(($label_k.into(), $label_v.into())),*]),
            )
        )
    };
}

fn heartbeat() -> Sample {
    sample_now(1.into())
}

// System info metrics
fn cpu_usage_percent(system: &mut System) -> Sample {
    system.refresh_cpu_specifics(CpuRefreshKind::nothing().with_cpu_usage());
    sample_now(system.global_cpu_usage().into())
}
fn ram_usage_bytes(system: &mut System) -> Sample {
    sample_now(system.used_memory() as f64)
}
fn ram_available_bytes(system: &mut System) -> Sample {
    sample_now(system.available_memory() as f64)
}

pub async fn consumer_loop(rx: &mut mpsc::UnboundedReceiver<TimeSeries>, config: MetricsConfig) {
    let chunk_size = 100;
    let mut buffer: Vec<TimeSeries> = Vec::with_capacity(chunk_size);
    let mut proto_encoded_buffer: Vec<u8> = Vec::new();
    let mut snap_encoder = Encoder::new();
    let client = reqwest::Client::new();

    loop {
        let open = rx.recv_many(&mut buffer, chunk_size).await;
        if open == 0 {
            break;
        }
        let write_request = WriteRequest {
            timeseries: buffer.clone(),
        };
        dbg!(&write_request.timeseries);

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
            }
        };
        buffer.clear();
        proto_encoded_buffer.clear();
    }
    println!("Producer dropped, consumer exiting");
}

pub async fn metrics_loop(config: IngestorConfig) {
    let (tx, mut rx) = mpsc::unbounded_channel::<TimeSeries>();

    let metrics: HashMap<String, (MetricFunc, MetricLabels)> = HashMap::from([
        // System info metrics
        system_metric!(
            cpu_usage_percent,
            [
                ("__name__", "cpu_usage_percent"),
                ("beamline", &config.loki.beamline_name)
            ]
        ),
        system_metric!(
            ram_usage_bytes,
            [
                ("__name__", "ram_usage_bytes"),
                ("beamline", &config.loki.beamline_name)
            ]
        ),
        system_metric!(
            ram_available_bytes,
            [
                ("__name__", "ram_available_bytes"),
                ("beamline", &config.loki.beamline_name)
            ]
        ),
    ]);

    let futs: Vec<JoinHandle<()>> = metrics
        .into_iter()
        .map(|(name, (func, labels))| {
            let interval = config.metrics.interval_for_metric(&name);
            spawn(metric_future(
                func,
                labels,
                interval,
                config.redis.clone(),
                tx.clone(),
            ))
        })
        .collect();

    consumer_loop(&mut rx, config.metrics.clone()).await;
    rx.close();
    // with rx closed, all futures should exit on their next iteration\, but some could be at too long
    // of an interval to wait for
    for handle in &futs {
        handle.abort(); // abort each task
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::metrics::{MetricFunc, cpu_usage_percent};

    #[test]
    fn test_system_metric_macro() {
        let (name, (_, labels)): (String, (MetricFunc, HashMap<String, String>)) =
            system_metric!(cpu_usage_percent, [("beamline", "x99xa")]);
        assert_eq!(name, "cpu_usage_percent");
        assert_eq!(labels.get("beamline").unwrap(), "x99xa");
    }
}
