use crate::{
    config::{IngestorConfig, MetricsConfig, RedisConfig},
    redis_logs::create_redis_conn,
};
use futures::future::join_all;
use redis::{Commands, Connection};
use std::collections::HashMap;
use sysinfo::System;
use tokio::{
    spawn,
    sync::mpsc::{self, UnboundedSender},
    task::JoinHandle,
    time,
};

pub mod prometheus {
    include!(concat!(env!("OUT_DIR"), "/prometheus.rs"));
}
use prometheus::{TimeSeries, WriteRequest};
type SimpleMetricFunc = Box<dyn Fn() -> TimeSeries + Send + Sync>;
type SysinfoMetricFunc = Box<dyn Fn(&System) -> TimeSeries + Send + Sync>;
type RedisMetricFunc = Box<dyn Fn(&mut Connection) -> TimeSeries + Send + Sync>;

enum MetricFunc {
    Simple(SimpleMetricFunc),
    Redis(RedisMetricFunc),
    System(SysinfoMetricFunc),
}

/// Create a future for a given metric function.
async fn metric_future(
    metric_func: MetricFunc,
    mut interval: time::Interval,
    redis_config: RedisConfig,
    tx: UnboundedSender<TimeSeries>,
) {
    match metric_func {
        MetricFunc::Simple(ref func) => loop {
            if tx.send((func())).is_err() {
                break;
            }
        },
        MetricFunc::Redis(ref func) => {
            let mut redis = create_redis_conn(&redis_config.url.full_url())
                .expect("Could not connect to Redis!");
            loop {
                if tx.send((func(&mut redis))).is_err() {
                    break;
                }
            }
        }
        MetricFunc::System(ref func) => {
            let system = System::new_all();
            loop {
                interval.tick().await;
                if tx.send((func(&system))).is_err() {
                    break;
                }
            }
        }
    };
}

// Macros create a hashmap entry from a conforming function
macro_rules! simple_metric {
    ($func:expr) => {
        (
            stringify!($func).to_string(),
            $crate::metrics::Simple(Box::new($func) as $crate::metrics::SimpleMetricFunc),
        )
    };
}
macro_rules! system_metric {
    ($func:expr) => {
        (
            stringify!($func).to_string(),
            $crate::metrics::MetricFunc::System(
                Box::new($func) as $crate::metrics::SysinfoMetricFunc
            ),
        )
    };
}
macro_rules! redis_metric {
    ($func:expr) => {
        (
            stringify!($func).to_string(),
            $crate::metrics::MetricFunc::Redis(Box::new($func) as $crate::metrics::RedisMetricFunc),
        )
    };
}

// System info metrics
fn cpu_usage_percent(system: &System) -> TimeSeries {
    system.global_cpu_usage().to_string()
}
fn ram_usage_bytes(system: &System) -> TimeSeries {
    system.used_memory().to_string()
}
fn ram_available_bytes(system: &System) -> TimeSeries {
    system.available_memory().to_string()
}

// Metrics from redis
fn redis_metric_1(redis: &mut Connection) -> TimeSeries {
    if let Ok(res) = redis.get::<&str, String>("key") {
        res
    } else {
        "ERROR".into()
    }
}

fn sort_updates(updates: Vec<TimeSeries>) -> WriteRequest {
    let mut map: HashMap<String, Vec<TimeSeries>> = HashMap::new();

    for item in updates {
        map.entry(item.0.clone()).or_default().push(item);
    }

    map.into_values().collect()
}

fn compile_message(updates: Vec<MetricUpdate>) -> prometheus::WriteRequest {
    todo!()
}

pub async fn consumer_loop(rx: &mut mpsc::UnboundedReceiver<MetricUpdate>, config: MetricsConfig) {
    let chunk_size = 100;
    let mut buffer: Vec<MetricUpdate> = Vec::with_capacity(chunk_size);
    let client = reqwest::Client::new();

    loop {
        let open = rx.recv_many(&mut buffer, chunk_size).await;
        if open == 0 {
            break;
        }
        match client
            .post(&config.url)
            .basic_auth(&config.auth.username, Some(&config.auth.password))
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
    }
    println!("Producer dropped, consumer exiting");
}

pub async fn metrics_loop(config: IngestorConfig) {
    let (tx, mut rx) = mpsc::unbounded_channel::<MetricUpdate>();

    let metrics: HashMap<String, MetricFunc> = HashMap::from([
        // System info metrics
        system_metric!(cpu_usage_percent),
        system_metric!(ram_usage_bytes),
        system_metric!(ram_available_bytes),
        // Redis metrics
        redis_metric!(redis_metric_1),
    ]);

    let futs: Vec<JoinHandle<()>> = metrics
        .into_iter()
        .map(|(name, func)| {
            let interval = config.metrics.interval_for_metric(&name);
            spawn(metric_future(
                name,
                func,
                interval,
                config.redis.clone(),
                tx.clone(),
            ))
        })
        .collect();

    let metric_futs = join_all(futs);
    consumer_loop(&mut rx, config.metrics.clone()).await;
    // try_join!(metric_futs);
}

#[cfg(test)]
mod tests {
    use crate::metrics::cpu_usage_percent;

    #[test]
    fn test_system_metric_macro() {
        let (name, _) = system_metric!(cpu_usage_percent);
        assert_eq!(name, "cpu_usage_percent")
    }
}
