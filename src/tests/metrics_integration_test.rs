use std::time::Duration;

use testcontainers_modules::{
    redis::Redis,
    testcontainers::{ContainerAsync, runners::AsyncRunner},
};
use tokio::spawn;

use crate::{config::IngestorConfig, metrics::metrics_loop};

// To run these tests, either a docker or podman socket should exist at $DOCKER_HOST or the default docker socket
// see https://rust.testcontainers.org/features/configuration/

async fn create_redis() -> (ContainerAsync<Redis>, String, u16) {
    let r = Redis::default().start().await.unwrap();
    let h: String = r.get_host().await.unwrap().to_string();
    let p: u16 = r.get_host_port_ipv4(6379).await.unwrap();
    (r, h, p)
}

fn config(redis_url: String, redis_port: u16, metrics_url: String) -> IngestorConfig {
    toml::from_str(
        format!(
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
        )
        .as_str(),
    )
    .unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn test_system_metrics_propagated() {
    tokio::time::timeout(Duration::from_secs(10), async {
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
            Box::leak(Box::new(config(redis_url, redis_port, server.url())));

        let handle = spawn(metrics_loop(config));
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
