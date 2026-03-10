use std::{panic::catch_unwind, time::Duration};

use redis::Commands;
use testcontainers_modules::{
    redis::Redis,
    testcontainers::{ContainerAsync, runners::AsyncRunner},
};
use tokio::{
    spawn,
    sync::mpsc::{self, Receiver, UnboundedReceiver},
    time::sleep,
};

use crate::{
    config::IngestorConfig,
    loki_push::consumer_loop,
    models::LogMsg,
    redis_logs::{RedisError, create_redis_conn_with_retry, producer_loop},
};

// To run these tests, either a docker or podman socket should exist at $DOCKER_HOST or the default docker socket
// see https://rust.testcontainers.org/features/configuration/

async fn create_redis() -> (ContainerAsync<Redis>, String, u16) {
    let r = Redis::default().start().await.unwrap();
    let h: String = r.get_host().await.unwrap().to_string();
    let p: u16 = r.get_host_port_ipv4(6379).await.unwrap();
    (r, h, p)
}

fn config(redis_url: String, redis_port: u16, service_url: String) -> IngestorConfig {
    toml::from_str(
        format!(
            r#"enable_metrics=false

[redis]
chunk_size = 10
blocktime_millis = 1000
consumer_group = "log-ingestor"
consumer_id = "log-ingestor"

[redis.url]
url = "redis://{redis_url}"
port = {redis_port}

[loki]
url = "{service_url}"
auth = {{ username = "test-loki", password = "test-loki-password" }}
chunk_size = 100
beamline_name = "x99xa"

[metrics]
url = "http://localhost"
auth = {{ username = "test-mimir", password = "test-mimir-password" }}

"#
        )
        .as_str(),
    )
    .unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn test_conn_no_redis() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let config: &'static mut IngestorConfig = Box::leak(Box::new(config(
            "redis://123.123.123.123".into(),
            12345,
            "".into(),
        )));
        let Err(result) = create_redis_conn_with_retry(config, 2, 10) else {
            panic!("shouldn't be able to connect")
        };
        assert_eq!(result, RedisError::Retryable("Code: unknown".into()));
    })
    .await
    .unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn test_loki_no_endpoint() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let (redis_container, redis_url, redis_port) = create_redis().await;
        let _ = redis_container.start().await;
        let config: &'static mut IngestorConfig =
            Box::leak(Box::new(config(redis_url, redis_port, "".into())));

        let (tx, _) = mpsc::unbounded_channel::<LogMsg>();

        let result = producer_loop(tx, config, 1, 10).await;
        assert_eq!(
            result,
            Err(RedisError::Retryable("No logging endpoint found".into()))
        );

        let _ = redis_container.stop_with_timeout(Some(0)).await;
    })
    .await
    .unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn test_loki_malformed() {
    let mut server = mockito::Server::new_async().await;

    let mock = server
        .mock("POST", "/")
        .match_body("{\"streams\":[{\"stream\":{\"hostname\":\"sparkle\",\"label\":\"bec_logs\",\"level\":\"ERROR\",\"service_name\":\"\"},\"values\":[[\"0\",\"Error processing log messages from Redis!\",{\"beamline_name\":\"x99xa\",\"exception\":\"None\",\"file_location\":\"\",\"file_name\":\"\",\"function\":\"\",\"line\":\"0\",\"module\":\"\",\"proc_id\":\"0\"}]]}]}")
        .create();

    let (redis_container, redis_url, redis_port) =
        tokio::time::timeout(Duration::from_secs(15), async { create_redis().await })
            .await
            .unwrap();
    let _ = redis_container.start().await;
    let config: &'static mut IngestorConfig =
        Box::leak(Box::new(config(redis_url, redis_port, server.url())));
    let mut conn = redis::Client::open(config.redis.url.full_url())
        .unwrap()
        .get_connection()
        .unwrap();
    let _: Result<String, _> = dbg!(conn.xadd("info/log", "*", &[("data", "test log")]));
    let (tx, rx) = mpsc::unbounded_channel::<LogMsg>();
    let rxbox: &'static mut UnboundedReceiver<_> = Box::leak(Box::new(rx));
    let producer = tokio::spawn(producer_loop(tx, config, 1, 10));
    let consumer = tokio::spawn(consumer_loop(rxbox, config));

    // Always wait for the whole timeout, not very good, but checking the mock is not supported
    // See https://github.com/lipanski/mockito/issues/227
    sleep(Duration::from_millis(3000)).await;

    consumer.abort();
    producer.abort();

    mock.assert();
    let _ = redis_container.stop_with_timeout(Some(0)).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_loki_happy_path() {
    let mut server = mockito::Server::new_async().await;

    let mock = server.mock("POST", "/").match_body("{\"streams\":[{\"stream\":{\"hostname\":\"sparkle\",\"label\":\"bec_logs\",\"level\":\"INFO\",\"service_name\":\"FileWriterManager\"},\"values\":[[\"1773154393820179968\",\"Waiting for DeviceServer.\",{\"beamline_name\":\"x99xa\",\"exception\":\"None\",\"file_location\":\"/home/david/Development/bec/bec/bec_lib/bec_lib/bec_service.py\",\"file_name\":\"bec_service.py\",\"function\":\"wait_for_service\",\"line\":\"434\",\"module\":\"bec_service\",\"proc_id\":\"222077\"}]]}]}").create();

    let (redis_container, redis_url, redis_port) =
        tokio::time::timeout(Duration::from_secs(15), async { create_redis().await })
            .await
            .unwrap();
    let _ = redis_container.start().await;
    let config: &'static mut IngestorConfig =
        Box::leak(Box::new(config(redis_url, redis_port, server.url())));
    let mut conn = redis::Client::open(config.redis.url.full_url())
        .unwrap()
        .get_connection()
        .unwrap();
    let _: Result<String, _> = dbg!(conn.xadd("info/log", "*", &[("data", b"\x81\xad__bec_codec__\x83\xacencoder_name\xaaBECMessage\xa9type_name\xaaLogMessage\xa4data\x83\xa8metadata\x80\xa8log_type\xa4info\xa7log_msg\x83\xa4text\xd9O2026-03-10 15:53:13 | bec_lib.bec_service | [INFO] | Waiting for DeviceServer.\n\xa6record\x8d\xa7elapsed\x82\xa4repr\xae0:00:00.025823\xa7seconds\xcb?\x9aqX1\xf0=\x14\xa9exception\xc0\xa5extra\x80\xa4file\x82\xa4name\xaebec_service.py\xa4path\xd9>/home/david/Development/bec/bec/bec_lib/bec_lib/bec_service.py\xa8function\xb0wait_for_service\xa5level\x83\xa4icon\xa6\xe2\x84\xb9\xef\xb8\x8f\xa4name\xa4INFO\xa2no\x14\xa4line\xcd\x01\xb2\xa7message\xb9Waiting for DeviceServer.\xa6module\xabbec_service\xa4name\xb3bec_lib.bec_service\xa7process\x82\xa2id\xce\x00\x03c}\xa4name\xabMainProcess\xa6thread\x82\xa2id\xcf\x00\x00\x7fSCq\xfb\x80\xa4name\xaaMainThread\xa4time\x82\xa4repr\xd9 2026-03-10 15:53:13.820180+01:00\xa9timestamp\xcbA\xdal\x0c\x16t}\xd4\xacservice_name\xb1FileWriterManager")]));
    let (tx, rx) = mpsc::unbounded_channel::<LogMsg>();
    let rxbox: &'static mut UnboundedReceiver<_> = Box::leak(Box::new(rx));
    let producer = tokio::spawn(producer_loop(tx, config, 1, 10));
    let consumer = tokio::spawn(consumer_loop(rxbox, config));

    // Always wait for the whole timeout, not very good, but checking the mock is not supported
    // See https://github.com/lipanski/mockito/issues/227
    sleep(Duration::from_millis(3000)).await;

    consumer.abort();
    producer.abort();

    mock.assert();
    let _ = redis_container.stop_with_timeout(Some(0)).await;
}
