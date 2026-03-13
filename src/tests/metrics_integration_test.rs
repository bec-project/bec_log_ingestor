use std::{collections::HashMap, sync::Arc, time::Duration};

use redis::Commands;
use testcontainers_modules::{
    redis::Redis,
    testcontainers::{ContainerAsync, runners::AsyncRunner},
};
use tokio::spawn;

use crate::{
    config::IngestorConfig,
    metrics::{metric_definitions, metrics_loop},
    metrics_core::{
        MetricDefinition, PinMetricResultFut, SimpleMetricFunc, numerical_sample_now,
        static_metric_def, sync_metric,
    },
};

// To run these tests, either a docker or podman socket should exist at $DOCKER_HOST or the default docker socket
// see https://rust.testcontainers.org/features/configuration/

async fn create_redis() -> (ContainerAsync<Redis>, String, u16) {
    let r = Redis::default().start().await.unwrap();
    let h: String = r.get_host().await.unwrap().to_string();
    let p: u16 = r.get_host_port_ipv4(6379).await.unwrap();
    (r, h, p)
}

fn config(
    redis_url: String,
    redis_port: u16,
    metrics_url: String,
    additional: &str,
) -> IngestorConfig {
    toml::from_str(
        (format!(
            r#"enable_logging=false

[redis]
chunk_size = 10
blocktime_millis = 1000
consumer_group = "log-ingestor"
consumer_id = "log-ingestor"

[redis.url]
url = "redis://{redis_url}"
port = {redis_port}

[loki]
url = "http://localhost"
auth = {{ username = "test-loki", password = "test-loki-password" }}
chunk_size = 100
beamline_name = "x99xa"

[metrics]
url = "{metrics_url}"
auth = {{ username = "test-mimir", password = "test-mimir-password" }}
watchdog_interval = {{ Secondly = 20 }}
publish_interval = {{ Millis = 5 }}

"#
        ) + additional)
            .as_str(),
    )
    .unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn test_system_metrics_propagated() {
    tokio::time::timeout(Duration::from_secs(15), async {
        let mut server = mockito::Server::new_async().await;
        // 3 system metrics should be gathered and sent, bundle depends on timing
        // Repetition time is longer than test timeout so the maximum should be 3
        // requests.
        let mock = server
            .mock("POST", "/")
            .expect_at_least(1)
            .expect_at_most(3)
            .create();
        let (redis_container, redis_url, redis_port) = create_redis().await;
        let _ = redis_container.start().await;
        let config: &'static mut IngestorConfig =
            Box::leak(Box::new(config(redis_url, redis_port, server.url(), "")));

        let handle = spawn(metrics_loop(config, metric_definitions(config)));
        while !mock.matched_async().await {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        handle.abort();
        let _ = redis_container.stop_with_timeout(Some(0)).await;
        let _ = handle.await;
        mock.assert(); // check no more came after aborting
    })
    .await
    .unwrap()
}

fn dummy_metric(_: &mut ()) -> PinMetricResultFut<'_> {
    sync_metric(Ok((numerical_sample_now(0.0), None)))
}

const ENCODED_METRIC: &[u8; 228] = b"\x81\xad__bec_codec__\x83\xacencoder_name\xaaBECMessage\xa9type_name\xb4DynamicMetricMessage\xa4data\x83\xa8metadata\x80\xa7metrics\x81\xa1a\x81\xad__bec_codec__\x83\xacencoder_name\xa9BaseModel\xa9type_name\xb8_FloatDynamicMetricValue\xa4data\x82\xa5value\xcb@\x16\x00\x00\x00\x00\x00\x00\xa9type_name\xa5float\xa9timestamp\xcbA\xdal\xf1v\x86L\xcd";

#[tokio::test(flavor = "multi_thread")]
async fn test_dynamic_metrics() {
    let mut server = mockito::Server::new_async().await;
    // the first mock will be filled before the second starts receiving calls
    let mock = server.mock("POST", "/").expect(1).create();
    let mock2 = server.mock("POST", "/").expect(1).create();

    let test = tokio::time::timeout(Duration::from_secs(15), async {
        let (redis_container, redis_url, redis_port) = create_redis().await;
        let _ = redis_container.start().await;
        let config: &'static mut IngestorConfig = Box::leak(Box::new(config(
            redis_url,
            redis_port,
            server.url(),
            "
[metrics.dynamic]
dynamic_metric = { read_type = \"PubSub\", key = \"info/dynamic_metrics/test\" }
",
        )));
        let mut metrics: HashMap<String, MetricDefinition> = HashMap::from([static_metric_def(
            Arc::new(dummy_metric) as SimpleMetricFunc,
            "dummy_metric",
            (config, None, None),
        )]);
        metrics.extend(
            config
                .metrics
                .dynamic
                .iter()
                .map(|(n, dm)| (n.to_owned(), MetricDefinition::Dynamic(dm.clone()))),
        );

        let handle = spawn(metrics_loop(config, std::sync::Arc::new(metrics)));
        tokio::time::sleep(Duration::from_millis(100)).await;
        let mut conn = redis::Client::open(config.redis.url.full_url())
            .unwrap()
            .get_connection()
            .unwrap();
        // first mock receives the dummy metric, everything is running
        while !mock.matched_async().await {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        assert!(!mock2.matched_async().await);
        // publish to the dynamic metric
        let _: Result<String, _> = dbg!(conn.publish("info/dynamic_metrics/test", ENCODED_METRIC));
        // then receive a second call
        while !mock2.matched_async().await {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        handle.abort();
        let _ = redis_container.stop_with_timeout(Some(0)).await;
        let _ = handle.await;
    })
    .await;

    mock2.assert(); // check no more came after finishing

    test.unwrap();
}
