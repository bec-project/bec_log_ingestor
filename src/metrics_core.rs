use crate::{
    config::{IngestorConfig, RedisConfig},
    redis_logs::create_redis_conn,
};

use redis::Connection;
use std::{collections::HashMap, sync::Arc};
use sysinfo::System;
use thiserror::Error;
use tokio::{
    spawn,
    sync::{Mutex, mpsc::UnboundedSender},
    task::JoinHandle,
    time,
};

pub mod prometheus {
    include!(concat!(env!("OUT_DIR"), "/prometheus.rs"));
}
use prometheus::{Label, Sample, TimeSeries};

#[derive(Error, Debug)]
pub enum MetricError {
    #[error("Temporary error: {0}")]
    Retryable(String),
    #[error("Fatal error: {0}")]
    Fatal(String),
}

pub(crate) type MetricFuncResult = Result<(Sample, Option<HashMap<String, String>>), MetricError>;
pub(crate) type SimpleMetricFunc = Arc<dyn Fn() -> MetricFuncResult + Send + Sync>;
pub(crate) type SysinfoMetricFunc = Arc<dyn Fn(&mut System) -> MetricFuncResult + Send + Sync>;
pub(crate) type RedisMetricFunc = Arc<dyn Fn(&mut Connection) -> MetricFuncResult + Send + Sync>;
pub(crate) type MetricLabels = HashMap<String, String>;
pub(crate) type MetricDefaultIntervalSeconds = Option<u32>;
pub(crate) type MetricDefinition = (MetricFunc, MetricLabels, MetricDefaultIntervalSeconds);
pub(crate) type MetricDefinitions = Arc<HashMap<String, MetricDefinition>>;
pub(crate) type MetricFutures = Arc<Mutex<HashMap<String, JoinHandle<()>>>>;

#[derive(Clone)]
pub(crate) enum MetricFunc {
    Simple(SimpleMetricFunc),
    Redis(RedisMetricFunc),
    System(SysinfoMetricFunc),
}

// Return a Prometheus Sample struct for the given value with a timestamp of the current UTC time
pub(crate) fn sample_now(value: f64) -> Sample {
    Sample {
        timestamp: chrono::Utc::now().timestamp_millis(),
        value,
    }
}
pub(crate) fn labels_from_hashmap(labels: &MetricLabels) -> Vec<Label> {
    labels
        .iter()
        .map(|(k, v)| Label {
            name: k.into(),
            value: v.into(),
        })
        .collect()
}

pub(crate) fn metric_spawner(
    tx: UnboundedSender<TimeSeries>,
    config: &'static IngestorConfig,
) -> impl FnMut((&String, &MetricDefinition)) -> (String, JoinHandle<()>) {
    move |(name, (func, labels, default_interval))| {
        (
            name.clone(),
            spawn(metric_future(
                func.clone(),
                labels.clone(),
                config.metrics.interval_for_metric(&name, *default_interval),
                config.redis.clone(),
                tx.clone(),
            )),
        )
    }
}

/// Takes a metric function, its arguments, its repetition interval, a clone of the channel transmitter, and an owned
/// hashmap of the associated labels, and returns a loop awaiting the interval ticks which sends a prometheus/mimir
/// TimeSeries to the channel, ready for the consumer loop in metrics.rs to bundle and send to the database
#[macro_export]
macro_rules! transmit_metric_loop {
    ($func:expr, ($($args:expr),*), $interval:expr, $tx:expr, $labels:expr) => {
        loop {
            $interval.tick().await;
            let metric_res = $func($($args),*);
            let (sample, opt_extra_labels) = match metric_res {
                Err(MetricError::Fatal(msg)) => {panic!("{msg}")},
                Err(MetricError::Retryable(msg)) => {println!("WARNING: {msg}"); continue},
                Ok(res) => {
                    res
                }
            };
            let mut owned_labels = $labels;
            if let Some(extra_labels) = opt_extra_labels {
                owned_labels.extend(extra_labels);
            }
            if $tx
                .send(TimeSeries {
                    labels: labels_from_hashmap(&owned_labels),
                    samples: vec![sample],
                })
                .is_err()
            {
                break;
            }
        }
    };
}

/// Create a future for a given metric function.
pub(crate) async fn metric_future(
    metric_func: MetricFunc,
    labels: HashMap<String, String>,
    mut interval: time::Interval,
    redis_config: RedisConfig,
    tx: UnboundedSender<TimeSeries>,
) {
    match metric_func {
        MetricFunc::Simple(func) => transmit_metric_loop!(func, (), interval, tx, labels.clone()),
        MetricFunc::Redis(func) => {
            let mut redis = create_redis_conn(&redis_config.url.full_url())
                .expect("Could not connect to Redis!");
            transmit_metric_loop!(func, (&mut redis), interval, tx, labels.clone())
        }
        MetricFunc::System(func) => {
            let mut system = System::new_all();
            transmit_metric_loop!(func, (&mut system), interval, tx, labels.clone())
        }
    };
}

/// These macros create an entry for the main MetricDefinitions hashmap (name: String, (func: MetricFunc, labels: HM<String, String>, default_interval_seconds: Option<u32>))
/// from a conforming function, automatically adding the '__name__' label for prometheus/mimir from the function name
///
/// E.g. for a metric function which takes no arguments:
/// ```
/// /// Metric which is always true
/// fn always_true() -> MetricFuncResult {
///     (sample_now(1.into()), None)
/// }
///
/// simple_metric!(always_true, [("extra_label_key": "extra_label_value")], None)
/// ```
/// will give you:
/// ```
/// (
///     "always_true",
///     (
///         MetricFunc::Simple(Arc::new(always_true as dyn Fn() -> MetricFuncResult + Send + Sync)),
///         HashMap::from([
///                         ("__name__".into(): "always_true".into()),
///                         ("extra_label_key".into(): "extra_label_value".into()),
///                       ]),
///         None,
///     )
/// )
/// ```
///
/// TODO: These can be simplified to a nested macro when the feature is stabilized: https://github.com/rust-lang/rust/issues/83527
/// TODO: also take the "beamline" label from the config.
#[macro_export]
macro_rules! simple_metric {
    ($func:expr, [ $(($label_k:expr, $label_v:expr)),*], $int:expr) => {
        (
            stringify!($func).to_string(),
            (
                $crate::metrics_core::MetricFunc::Simple(std::sync::Arc::new($func) as $crate::metrics_core::SimpleMetricFunc),
                std::collections::HashMap::from([("__name__".to_string(), stringify!($func).to_string()), $(($label_k.into(), $label_v.into())),*]),
                $int
            )
        )
    };
}
#[macro_export]
macro_rules! system_metric {
    ($func:expr, [ $(($label_k:expr, $label_v:expr)),*], $int:expr) => {
        (
            stringify!($func).to_string(),
            (
                $crate::metrics_core::MetricFunc::System(std::sync::Arc::new($func) as $crate::metrics_core::SysinfoMetricFunc),
                std::collections::HashMap::from([("__name__".to_string(), stringify!($func).to_string()), $(($label_k.into(), $label_v.into())),*]),
                $int
            ),
        )
    };
}
#[macro_export]
macro_rules! redis_metric {
    ($func:expr, [ $(($label_k:expr, $label_v:expr)),*], $int:expr) => {
        (
            stringify!($func).to_string(),
            (
                $crate::metrics_core::MetricFunc::Redis(std::sync::Arc::new($func) as $crate::metrics_core::RedisMetricFunc),
                std::collections::HashMap::from([("__name__".to_string(), stringify!($func).to_string()), $(($label_k.into(), $label_v.into())),*]),
                $int
            )
        )
    };
}

#[cfg(test)]
mod tests {
    use crate::{config::MetricInterval, metrics_core::*};
    use std::{collections::HashMap, time::Duration};
    use sysinfo::System;
    use tokio::{sync::mpsc, time::interval};

    fn test_metric(system: &mut System) -> MetricFuncResult {
        Ok((
            Sample {
                value: 12.34,
                timestamp: 5678,
            },
            None,
        ))
    }

    fn test_config() -> &'static mut IngestorConfig {
        let test_str = "
[redis.url]
url = \"http://127.0.0.1\"
port = 12345

[loki]
auth = { username = \"test_user\", password = \"test_password\" }
url = \"http://127.0.0.1/api/v1/push\"

[metrics]
auth = { username = \"test_user\", password = \"test_password\" }
url = \"http://127.0.0.1\"
";
        let cfg: IngestorConfig = toml::from_str(&test_str).unwrap();
        Box::leak(Box::new(cfg))
    }

    #[test]
    fn test_system_metric_macro() {
        let (name, (func, labels, int)): (String, MetricDefinition) =
            system_metric!(test_metric, [("beamline", "x99xa")], None);
        assert_eq!(name, "test_metric");
        assert_eq!(labels.get("beamline").unwrap(), "x99xa");

        let mut system = System::new();
        match func {
            MetricFunc::Simple(_) => assert!(false),
            MetricFunc::Redis(_) => assert!(false),
            MetricFunc::System(func) => {
                let (samp, extra_labels) = func(&mut system).unwrap();
                assert!(extra_labels.is_none());
                assert_eq!(samp.value, 12.34);
                assert_eq!(samp.timestamp, 5678);
            }
        }
    }

    #[test]
    fn test_sample_now() {
        let samp = sample_now(0.0);
        let ts = chrono::Utc::now().timestamp_millis();
        dbg!(&samp.timestamp);
        dbg!(ts);
        assert!(samp.timestamp > (ts - 10_000)); // timestamp should be from the last ten seconds...
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_metric_future() {
        let mut config = test_config();
        config
            .metrics
            .intervals
            .insert("test_metric".into(), MetricInterval::Millis(1));
        let (tx, mut rx) = mpsc::unbounded_channel::<TimeSeries>();
        let mut spawner = metric_spawner(tx.clone(), config);

        let (name, (func, labels, int)): (String, MetricDefinition) =
            system_metric!(test_metric, [("beamline", "x99xa")], None);

        let (name, fut) = spawner((&name, &(func, labels, None)));
        let mut buf: Vec<TimeSeries> = Vec::with_capacity(10);
        rx.recv_many(&mut buf, 10).await;
        fut.abort();

        let item = buf.get(0).unwrap();
        assert_eq!(&item.labels.len(), &2);
        let labels: MetricLabels = item
            .labels
            .iter()
            .map(|l| (l.name.clone(), l.value.clone()))
            .collect();
        assert_eq!(labels.get("__name__").unwrap(), &"test_metric".to_string());
        assert_eq!(labels.get("beamline").unwrap(), &"x99xa".to_string());
    }
}
