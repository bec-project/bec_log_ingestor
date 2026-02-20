use crate::config::{
    DynamicMetric, DynamicMetricDtype, IngestorConfig, RedisConfig, RedisReadType,
};

use futures::StreamExt;
use redis::{AsyncCommands, Msg, RedisError, aio::MultiplexedConnection};
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

pub trait AsyncFnMut<A>: Send + Sync {
    fn call<'a>(&self, args: &'a mut A) -> PinMetricResultFut<'a>;
}

// #[async_trait]
impl<F, A> AsyncFnMut<A> for F
where
    F: Fn(&mut A) -> PinMetricResultFut + Send + Sync,
    A: Send,
{
    fn call<'a>(&self, args: &'a mut A) -> PinMetricResultFut<'a> {
        (self)(args)
    }
}

pub(crate) type MetricFuncResult<'a> =
    Result<(Sample, Option<HashMap<String, String>>), MetricError>;
pub(crate) type PinMetricResultFut<'a> =
    Pin<Box<dyn Future<Output = MetricFuncResult<'a>> + Send + 'a>>;
pub(crate) type SimpleMetricFunc = Arc<dyn AsyncFnMut<()> + Send>;
pub(crate) type SysMetricFunc = Arc<dyn AsyncFnMut<System> + Send>;
pub(crate) type RedisMetricFunc = Arc<dyn AsyncFnMut<MultiplexedConnection> + Send>;

pub(crate) type MetricLabels = HashMap<String, String>;
pub(crate) type MetricDefaultIntervalSeconds = Option<u32>;
pub(crate) type StaticMetricDefinition =
    (StaticMetricFunc, MetricLabels, MetricDefaultIntervalSeconds);
pub(crate) enum MetricDefinition {
    Static(StaticMetricDefinition),
    Dynamic(DynamicMetric),
}
pub(crate) type MetricDefinitions = Arc<HashMap<String, MetricDefinition>>;
pub(crate) type MetricFutures = Arc<Mutex<HashMap<String, JoinHandle<()>>>>;

#[derive(Clone)]
pub(crate) enum StaticMetricFunc {
    Simple(SimpleMetricFunc),
    Redis(RedisMetricFunc),
    System(SysMetricFunc),
}

impl From<SimpleMetricFunc> for StaticMetricFunc {
    fn from(value: SimpleMetricFunc) -> Self {
        StaticMetricFunc::Simple(value)
    }
}
impl From<SysMetricFunc> for StaticMetricFunc {
    fn from(value: SysMetricFunc) -> Self {
        StaticMetricFunc::System(value)
    }
}
impl From<RedisMetricFunc> for StaticMetricFunc {
    fn from(value: RedisMetricFunc) -> Self {
        StaticMetricFunc::Redis(value)
    }
}

pub(crate) fn sync_metric(result: MetricFuncResult) -> PinMetricResultFut {
    Box::pin(async move { result })
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
        let metric_res = func.call(&mut args).await;
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

fn parse_redis_value(config: &DynamicMetric, redis_value: String) -> MetricFuncResult<'_> {
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

fn dynamic_polling_metric<'a>(
    (redis, config): &'a mut (MultiplexedConnection, DynamicMetric),
) -> Pin<Box<dyn Future<Output = MetricFuncResult<'a>> + Send + 'a>> {
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
            let client = redis::Client::open(redis_config.url.full_url())
                .expect("Could not connect to redis!");
            let mut pubsub = client
                .get_async_pubsub()
                .await
                .expect("Could not get pubsub client!");
            pubsub
                .subscribe(&config.key)
                .await
                .expect("Could not subscribe to channel!");
            pubsub
                .on_message()
                .map(|msg: Msg| {
                    let payload: String = msg.get_payload().expect("Failed to extract payload!");
                    let (sample, opt_extra_labels) =
                        match parse_redis_value(&config, payload.into()) {
                            Err(MetricError::Fatal(error_msg)) => {
                                panic!("FATAL: {error_msg}")
                            }
                            Err(MetricError::Retryable(error_msg)) => {
                                println!("WARNING: {error_msg}");
                                return ();
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
                        panic!("TASK-FATAL: error transmitting metric to publisher channel");
                    }
                })
                .collect::<()>()
                .await
        }
        RedisReadType::Poll(interval_config) => {
            let c = config.clone();
            polling_metric_loop(
                Arc::new(dynamic_polling_metric),
                (redis.clone(), c),
                interval_config.into(),
                tx,
                labels,
            )
            .await
        }
    }
}

pub(crate) fn static_metric_def<F>(
    func: F,
    config: &'static IngestorConfig,
    default_interval: MetricDefaultIntervalSeconds,
    extra_labels: Option<&[(&str, &str)]>,
) -> (String, MetricDefinition)
where
    F: Into<StaticMetricFunc>,
{
    let mut metric_labels: MetricLabels =
        HashMap::from([("beamline".into(), config.loki.beamline_name.to_owned())]);
    metric_labels.insert("__name__".into(), stringify!(func).into());
    if let Some(extend_labels) = extra_labels {
        metric_labels.extend(extend_labels.iter().map(|&(a, b)| (a.into(), b.into())));
    }
    (
        stringify!(func).into(),
        MetricDefinition::Static((func.into(), metric_labels, default_interval)),
    )
}

#[cfg(test)]
mod tests {
    use crate::metrics_core::*;
    use sysinfo::System;

    fn test_metric(_system: &mut System) -> PinMetricResultFut<'_> {
        Box::pin(async move {
            Ok((
                Sample {
                    value: 12.34,
                    timestamp: 5678,
                },
                None,
            ))
        })
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
    fn test_sample_now() {
        let samp = sample_now(0.0);
        let ts = chrono::Utc::now().timestamp_millis();
        dbg!(&samp.timestamp);
        dbg!(ts);
        assert!(samp.timestamp > (ts - 10_000)); // timestamp should be from the last ten seconds...
    }

    // Need to mock/trait redis to re-enable this test
    // #[tokio::test(flavor = "multi_thread")]
    // async fn test_metric_future() {
    //     let mut config = test_config();
    //     config
    //         .metrics
    //         .intervals
    //         .insert("test_metric".into(), MetricInterval::Millis(1));
    //     let (tx, mut rx) = mpsc::unbounded_channel::<TimeSeries>();
    //     let mut spawner = metric_spawner(tx.clone(), config.clone(), todo!());

    //     let (name, def): (String, MetricDefinition) =
    //         system_metric!(test_metric, [("beamline", "x99xa")], None);
    //     // Assert that the type is a static metric definition and then rewrap it
    //     let MetricDefinition::Static((func, labels, int)) = def else {
    //         panic!()
    //     };
    //     let (_, fut) = spawner((&name, &MetricDefinition::Static((func, labels, None))));
    //     let mut buf: Vec<TimeSeries> = Vec::with_capacity(10);
    //     rx.recv_many(&mut buf, 10).await;
    //     fut.abort();

    //     let item = buf.get(0).unwrap();
    //     assert_eq!(&item.labels.len(), &2);
    //     let labels: MetricLabels = item
    //         .labels
    //         .iter()
    //         .map(|l| (l.name.clone(), l.value.clone()))
    //         .collect();
    //     assert_eq!(labels.get("__name__").unwrap(), &"test_metric".to_string());
    //     assert_eq!(labels.get("beamline").unwrap(), &"x99xa".to_string());
    // }
}
