use crate::config::{
    DynamicMetric, DynamicMetricDtype, IngestorConfig, RedisConfig, RedisReadType,
};

use async_trait::async_trait;
use redis::{AsyncCommands, RedisError, aio::MultiplexedConnection};
use std::{collections::HashMap, pin::Pin, sync::Arc};
use sysinfo::System;
use thiserror::Error;
use tokio::{
    spawn,
    sync::{Mutex, mpsc::UnboundedSender},
    task::JoinHandle,
    time::{self, Interval},
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

// #[async_trait]
pub trait AsyncFnMut<A>: Send + Sync {
    fn call(&self, args: &mut A) -> Pin<Box<dyn Future<Output = MetricFuncResult> + Send>>;
}

// #[async_trait]
impl<F, A> AsyncFnMut<A> for F
where
    F: Fn(&mut A) -> Pin<Box<dyn Future<Output = MetricFuncResult> + Send>> + Send + Sync,
    A: Send,
{
    fn call(&self, args: &mut A) -> Pin<Box<dyn Future<Output = MetricFuncResult> + Send>> {
        (self)(args)
    }
}

pub(crate) type MetricFuncResult = Result<(Sample, Option<HashMap<String, String>>), MetricError>;
pub(crate) type SimpleMetricFunc = Arc<dyn AsyncFnMut<()> + Send>;
pub(crate) type SysinfoMetricFunc = Arc<dyn AsyncFnMut<System> + Send>;
pub(crate) type RedisMetricFunc = Arc<dyn AsyncFnMut<MultiplexedConnection> + Send>;

pub(crate) type MetricLabels = HashMap<String, String>;
pub(crate) type MetricDefaultIntervalSeconds = Option<u32>;
pub(crate) type StaticMetricDefinition =
    (StaticMetricFunc, MetricLabels, MetricDefaultIntervalSeconds);
pub(crate) enum MetricDefinition {
    Static(StaticMetricDefinition),
    Dynamic(&'static DynamicMetric),
}
pub(crate) type MetricDefinitions = Arc<HashMap<String, MetricDefinition>>;
pub(crate) type MetricFutures = Arc<Mutex<HashMap<String, JoinHandle<()>>>>;

#[derive(Clone)]
pub(crate) enum StaticMetricFunc {
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
    config: IngestorConfig,
    redis: MultiplexedConnection,
) -> impl FnMut((&String, &MetricDefinition)) -> (String, JoinHandle<()>) {
    move |(name, definition)| match definition {
        MetricDefinition::Static((func, labels, default_interval)) => (
            name.clone(),
            spawn(metric_future(
                func.clone(),
                labels.clone(),
                config.metrics.interval_for_metric(&name, *default_interval),
                redis.clone(),
                tx.clone(),
            )),
        ),
        MetricDefinition::Dynamic(metric_config) => (
            name.clone(),
            spawn(dynamic_metric_future(
                HashMap::from([
                    ("__name__".to_string(), name.clone()),
                    ("beamline".into(), config.loki.beamline_name.to_owned()),
                ]),
                metric_config.clone().clone(),
                redis.clone(),
                config.redis.clone(),
                tx.clone(),
            )),
        ),
    }
}

/// Takes a metric function, its arguments, its repetition interval, a clone of the channel transmitter, and an owned
/// hashmap of the associated labels, and runs a loop awaiting the interval ticks which sends a prometheus/mimir
/// TimeSeries to the channel, ready for the consumer loop in metrics.rs to bundle and send to the database
async fn polling_metric_loop<Args>(
    func: Arc<dyn AsyncFnMut<Args>>,
    mut args: Args,
    mut interval: Interval,
    tx: UnboundedSender<TimeSeries>,
    labels: MetricLabels,
) -> () {
    loop {
        interval.tick().await;
        let metric_res = Box::pin(func.call(&mut args)).await;
        let (sample, opt_extra_labels) = match metric_res {
            Err(MetricError::Fatal(msg)) => {
                panic!("FATAL: {msg}")
            }
            Err(MetricError::Retryable(msg)) => {
                println!("WARNING: {msg}");
                continue;
            }
            Ok(res) => res,
        };
        let mut owned_labels = labels.clone();
        if let Some(extra_labels) = opt_extra_labels {
            owned_labels.extend(extra_labels);
        }
        if tx
            .send(TimeSeries {
                labels: labels_from_hashmap(&owned_labels),
                samples: vec![sample],
            })
            .is_err()
        {
            break;
        }
    }
}

/// Create a future for a given metric function.
pub(crate) async fn metric_future(
    metric_func: StaticMetricFunc,
    labels: HashMap<String, String>,
    interval: time::Interval,
    redis: MultiplexedConnection,
    tx: UnboundedSender<TimeSeries>,
) {
    match metric_func {
        StaticMetricFunc::Simple(func) => polling_metric_loop(func, (), interval, tx, labels).await,
        StaticMetricFunc::Redis(func) => {
            polling_metric_loop(func, redis, interval, tx, labels).await
        }
        StaticMetricFunc::System(func) => {
            let system = System::new_all();
            polling_metric_loop(func, system, interval, tx, labels).await
        }
    }
}

fn parsing_error<T: std::fmt::Debug>(
    config: &DynamicMetric,
    val: T,
    detail: String,
) -> MetricError {
    MetricError::Retryable(format!(
        "Could not parse dynamically defined metric at: {} as a {} value: {val:?}. Detail: {detail}",
        config.key, config.dtype
    ))
}

fn parse_redis_value(config: &DynamicMetric, redis_value: String) -> MetricFuncResult {
    match config.dtype {
        DynamicMetricDtype::String => Ok((
            sample_now(1.0),
            Some(HashMap::from([("value".into(), redis_value)])),
        )),
        DynamicMetricDtype::Float => {
            let res = redis_value
                .parse::<f64>()
                .map_err(|e| parsing_error(&config, &redis_value, e.to_string()))?;
            Ok((sample_now(res), None))
        }
        DynamicMetricDtype::Int => {
            let res = redis_value
                .parse::<i64>()
                .map_err(|e| parsing_error(&config, &redis_value, e.to_string()))?;
            Ok((sample_now(res as f64), None))
        }
    }
}

fn sync_get(
    mut redis: MultiplexedConnection,
    key: String,
) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, RedisError>> + Send>> {
    Box::pin(async move { redis.get(key).await })
}

fn dynamic_polling_metric(
    (redis, config): &mut (MultiplexedConnection, DynamicMetric),
) -> Pin<Box<dyn Future<Output = MetricFuncResult> + Send>> {
    Box::pin(async move {
        let val: Vec<u8> = sync_get(redis.clone(), (&config.key).to_owned())
            .await
            .expect("Unspecified redis error");
        if val.is_empty() {
            Err(MetricError::Retryable(format!(
                "No value found at dynamically defined key: {:?}",
                &config.key
            )))
        } else {
            let res_str =
                str::from_utf8(&val).map_err(|e| parsing_error(&config, &val, e.to_string()))?;
            parse_redis_value(config, res_str.into())
        }
    })
}

/// Create a future for a given dynamic metric configuration.
pub(crate) async fn dynamic_metric_future(
    labels: HashMap<String, String>,
    config: DynamicMetric,
    redis: MultiplexedConnection,
    redis_config: RedisConfig,
    tx: UnboundedSender<TimeSeries>,
) {
    match &config.read_type {
        RedisReadType::PubSub => {
            todo!()
            // let mut pubsub = redis.get_async_pubsub();
            // pubsub
            //     .subscribe(&config.key)
            //     .expect("Could not subscribe to channel!");
            // loop {
            //     let msg = pubsub.get_message().expect("Failed to get message!");
            //     let payload: String = msg.get_payload().expect("Failed to extract payload!");
            //     let (sample, opt_extra_labels) = match parse_redis_value(&config, payload.into()) {
            //         Err(MetricError::Fatal(msg)) => {
            //             panic!("FATAL: {msg}")
            //         }
            //         Err(MetricError::Retryable(msg)) => {
            //             println!("WARNING: {msg}");
            //             continue;
            //         }
            //         Ok(res) => res,
            //     };
            //     let mut owned_labels = dbg!(labels.clone());
            //     dbg!(sample);
            //     if let Some(extra_labels) = opt_extra_labels {
            //         owned_labels.extend(extra_labels);
            //     }
            //     if tx
            //         .send(TimeSeries {
            //             labels: labels_from_hashmap(&owned_labels),
            //             samples: vec![sample],
            //         })
            //         .is_err()
            //     {
            //         println!("TASK-FATAL: error transmitting metric to publisher channel");
            //         break;
            //     }
            // }
        }
        RedisReadType::Poll(interval_config) => {
            let polling_interval: Interval = interval_config.into();
            polling_metric_loop(
                Arc::new(dynamic_polling_metric),
                (redis.clone(), config),
                polling_interval,
                tx,
                labels,
            )
            .await
        }
    }
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
                $crate::metrics_core::StaticMetricFunc::Simple(std::sync::Arc::new($func) as $crate::metrics_core::SimpleMetricFunc),
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
            $crate::metrics_core::MetricDefinition::Static((
                $crate::metrics_core::StaticMetricFunc::System(std::sync::Arc::new($func)),
                std::collections::HashMap::from([("__name__".to_string(), stringify!($func).to_string()), $(($label_k.into(), $label_v.into())),*]),
                $int
            )),
        )
    };
}
#[macro_export]
macro_rules! redis_metric {
    ($func:expr, [ $(($label_k:expr, $label_v:expr)),*], $int:expr) => {
        (
            stringify!($func).to_string(),
            $crate::metrics_core::MetricDefinition::Static((
                $crate::metrics_core::StaticMetricFunc::Redis(std::sync::Arc::new($func) as $crate::metrics_core::RedisMetricFunc),
                std::collections::HashMap::from([("__name__".to_string(), stringify!($func).to_string()), $(($label_k.into(), $label_v.into())),*]),
                $int
            ))
        )
    };
}

#[cfg(test)]
mod tests {
    use crate::{config::MetricInterval, metrics_core::*};
    use sysinfo::System;
    use tokio::sync::mpsc;

    fn test_metric(_system: &mut System) -> MetricFuncResult {
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
        let (name, def): (String, MetricDefinition) =
            system_metric!(test_metric, [("beamline", "x99xa")], None);
        let MetricDefinition::Static((func, labels, int)) = def else {
            panic!()
        };
        assert_eq!(name, "test_metric");
        assert_eq!(labels.get("beamline").unwrap(), "x99xa");

        let mut system = System::new();
        match func {
            StaticMetricFunc::Simple(_) => assert!(false),
            StaticMetricFunc::Redis(_) => assert!(false),
            StaticMetricFunc::System(func) => {
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

        let (name, def): (String, MetricDefinition) =
            system_metric!(test_metric, [("beamline", "x99xa")], None);
        // Assert that the type is a static metric definition and then rewrap it
        let MetricDefinition::Static((func, labels, int)) = def else {
            panic!()
        };
        let (_, fut) = spawner((&name, &MetricDefinition::Static((func, labels, None))));
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
