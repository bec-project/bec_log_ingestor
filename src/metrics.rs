use crate::{
    config::{MetricsConfig, RedisConfig},
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

type SimpleMetricFunc = Box<dyn Fn() -> String + Send + Sync>;
type SysinfoMetricFunc = Box<dyn Fn(&System) -> String + Send + Sync>;
type RedisMetricFunc = Box<dyn Fn(&mut Connection) -> String + Send + Sync>;
type MetricUpdate = (String, String);

enum MetricFunc {
    Simple(SimpleMetricFunc),
    Redis(RedisMetricFunc),
    System(SysinfoMetricFunc),
}

async fn metric_future(
    name: String,
    metric_func: MetricFunc,
    mut interval: time::Interval,
    redis_config: RedisConfig,
    tx: UnboundedSender<MetricUpdate>,
) {
    match metric_func {
        MetricFunc::Simple(ref func) => loop {
            if tx.send(("".into(), func())).is_err() {
                break;
            }
        },
        MetricFunc::Redis(ref func) => {
            let mut redis = create_redis_conn(&redis_config.url.full_url())
                .expect("Could not connect to Redis!");
            loop {
                if tx.send(("".into(), func(&mut redis))).is_err() {
                    break;
                }
            }
        }
        MetricFunc::System(ref func) => {
            let system = System::new_all();
            loop {
                interval.tick().await;
                if tx.send(("".into(), func(&system))).is_err() {
                    break;
                }
            }
        }
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
fn cpu_usage_percent(system: &System) -> String {
    system.global_cpu_usage().to_string()
}
fn ram_usage_bytes(system: &System) -> String {
    system.used_memory().to_string()
}
fn ram_available_bytes(system: &System) -> String {
    system.available_memory().to_string()
}

// Metrics from redis
fn redis_metric_1(redis: &mut Connection) -> String {
    if let Ok(res) = redis.get::<&str, String>("key") {
        res
    } else {
        "ERROR".into()
    }
}

pub async fn metrics_loop(config: MetricsConfig, redis_config: RedisConfig) {
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
            let interval = config.interval_for_metric(&name);
            spawn(metric_future(
                name,
                func,
                interval,
                redis_config.clone(),
                tx.clone(),
            ))
        })
        .collect();

    join_all(futs).await;
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
