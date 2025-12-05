use crate::config::MetricsConfig;
use futures::future::join_all;
use std::collections::HashMap;
use sysinfo::System;
use tokio::{spawn, sync::mpsc, task::JoinHandle, time};

type SimpleMetricFunc = Box<dyn Fn() -> String + Send + Sync>;
type SysinfoMetricFunc = Box<dyn Fn(&System) -> String + Send + Sync>;
type RedisMetricFunc = Box<dyn Fn(i32) -> String + Send + Sync>;
enum MetricFunc {
    Simple(SimpleMetricFunc),
    Redis(RedisMetricFunc),
    System(SysinfoMetricFunc),
}

async fn metric_future(metric_func: MetricFunc, mut interval: time::Interval) {
    let system = System::new_all();
    loop {
        interval.tick().await;

        match metric_func {
            MetricFunc::Simple(ref func) => func(),
            MetricFunc::Redis(ref func) => {
                let redis: i32 = 5;
                func(redis)
            }
            MetricFunc::System(ref func) => func(&system),
        };
    }
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

fn cpu_usage_percent(system: &System) -> String {
    system.global_cpu_usage().to_string()
}
fn ram_usage_bytes(system: &System) -> String {
    system.used_memory().to_string()
}
fn ram_available_bytes(system: &System) -> String {
    system.available_memory().to_string()
}

pub async fn metrics_loop(config: MetricsConfig) {
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    let metrics: HashMap<String, MetricFunc> = HashMap::from([
        system_metric!(cpu_usage_percent),
        system_metric!(ram_usage_bytes),
        system_metric!(ram_available_bytes),
    ]);

    let futs: Vec<JoinHandle<()>> = metrics
        .into_iter()
        .map(|(name, func)| spawn(metric_future(func, config.interval_for_metric(&name))))
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
