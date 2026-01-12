use std::{collections::HashMap, fmt, io::Read, time::Duration};

use serde_derive::{Deserialize, Serialize};

pub trait FromTomlFile {
    /// Parse a toml file for an IngestorConfig. Assumes the file exists and is readable.
    fn from_file(path: std::path::PathBuf) -> Self
    where
        Self: for<'de> serde::Deserialize<'de>,
    {
        let mut file = std::fs::File::open(path).expect("Cannot open supplied config file!");
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .expect("Cannot read supplied config file!");
        let self_name = std::any::type_name::<Self>();
        toml::from_str(&contents).unwrap_or_else(|_| panic!("Invalid TOML for {self_name} struct"))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UrlPort {
    pub url: String,
    pub port: u16,
}

impl UrlPort {
    pub fn full_url(&self) -> String {
        self.url.to_owned() + ":" + &self.port.to_string()
    }
}

#[derive(Clone, Deserialize)]

pub struct BasicAuth {
    pub username: String,
    pub password: String,
}

impl fmt::Debug for BasicAuth {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Basic auth: provided",)
    }
}

/// Default number of records to read from Redis or push to Loki at once
fn default_chunk_size() -> u16 {
    100
}
/// Default timeout for blocking XREAD calls
fn default_blocktime_millis() -> usize {
    1000
}
/// Default value for both the consumer group and consumer ID
fn default_consumer() -> String {
    "log-ingestor".into()
}
/// Default value for the beamline name
fn default_beamline_name() -> String {
    "x99xa".into()
}

#[derive(Clone, Debug, Deserialize)]
pub struct RedisConfig {
    pub url: UrlPort,
    #[serde(default = "default_chunk_size")]
    pub chunk_size: u16,
    #[serde(default = "default_blocktime_millis")]
    pub blocktime_millis: usize,
    #[serde(default = "default_consumer")]
    pub consumer_group: String,
    #[serde(default = "default_consumer")]
    pub consumer_id: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct LokiConfig {
    pub url: String,
    pub auth: BasicAuth,
    #[serde(default = "default_chunk_size")]
    pub chunk_size: u16,
    #[serde(default = "default_beamline_name")]
    pub beamline_name: String,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum MetricInterval {
    Millis(u64),
    Secondly(u64),
    Minutely(u64),
    Hourly(u64),
    Daily(u64),
    Weekly(u64),
}
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct MetricIntervalsConfig {
    #[serde(default = "HashMap::new")]
    pub intervals: HashMap<String, MetricInterval>,
}
impl FromTomlFile for MetricIntervalsConfig {}

impl Into<tokio::time::Interval> for &MetricInterval {
    fn into(self) -> tokio::time::Interval {
        match self {
            MetricInterval::Millis(n) => tokio::time::interval(Duration::from_millis(*n)),
            MetricInterval::Secondly(n) => tokio::time::interval(Duration::from_secs(*n)),
            MetricInterval::Minutely(n) => tokio::time::interval(Duration::from_secs(n * 60)),
            MetricInterval::Hourly(n) => tokio::time::interval(Duration::from_secs(n * 3600)),
            MetricInterval::Daily(n) => tokio::time::interval(Duration::from_secs(n * 86400)),
            MetricInterval::Weekly(n) => tokio::time::interval(Duration::from_secs(n * 604800)),
        }
    }
}

#[derive(Clone, Deserialize, Debug)]
pub struct MetricsConfig {
    pub user_config_path: Option<std::path::PathBuf>,
    pub auth: BasicAuth,
    pub url: String,
    #[serde(default = "HashMap::new")]
    pub intervals: HashMap<String, MetricInterval>,
}

impl MetricsConfig {
    /// Get an interval future for `metric_name`, defaults to one minute if unconfigured.
    pub fn interval_for_metric(&self, metric_name: &String) -> tokio::time::Interval {
        match self.intervals.get(metric_name) {
            None => tokio::time::interval(Duration::from_secs(60)),
            Some(int) => int.into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct IngestorConfig {
    pub redis: RedisConfig,
    pub loki: LokiConfig,
    pub metrics: MetricsConfig,
}
impl FromTomlFile for IngestorConfig {}

/// Assemble a full IngestorConfig from the paths from commandline arguments
/// The path for the main config must be supplied.
///
/// Metric interval configuration can be supplied in three places, in decreasing order of preference:
/// - in a file with the commandline -m flag
/// - in a file referenced as `metrics.user_config_path` in the main config file
/// - directly under [metrics.intervals] in the main config file
///
/// The file referenced in the main config doesn't have to exist, if not, it will be ignored
pub fn assemble_config(paths: (std::path::PathBuf, Option<std::path::PathBuf>)) -> IngestorConfig {
    let (path, metrics_path) = paths;
    let mut config = IngestorConfig::from_file(path);
    if let Some(ref intervals_reference_file) = config.metrics.user_config_path {
        if intervals_reference_file.exists() {
            let intervals = MetricIntervalsConfig::from_file(intervals_reference_file.clone());
            config.metrics.intervals.extend(intervals.intervals);
        } else {
            println!("Metric intervals config file not found at {intervals_reference_file:?}")
        }
    }
    if let Some(intervals) = metrics_path.map(MetricIntervalsConfig::from_file) {
        config.metrics.intervals.extend(intervals.intervals);
    }
    config
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_port() {
        let test_str = "
url = \"http://127.0.0.1\"
port = 12345
";
        let url: UrlPort = toml::from_str(&test_str).unwrap();
        assert_eq!(url.full_url(), "http://127.0.0.1:12345")
    }

    #[test]
    fn test_redis() {
        let test_str = "
[url]
url = \"http://127.0.0.1\"
port = 12345
";
        let redis: RedisConfig = toml::from_str(&test_str).unwrap();
        assert_eq!(redis.url.full_url(), "http://127.0.0.1:12345")
    }

    #[test]
    fn test_ingestor() {
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
        let config: IngestorConfig = toml::from_str(&test_str).unwrap();
        assert_eq!(config.redis.url.full_url(), "http://127.0.0.1:12345");
        assert_eq!(config.loki.url, "http://127.0.0.1/api/v1/push");
        assert_eq!(config.loki.auth.username, "test_user");
        assert_eq!(config.loki.auth.password, "test_password");
    }

    #[test]
    fn test_redis_defaults() {
        let test_str = "
[url]
url = \"http://localhost\"
port = 6379
";
        let redis: RedisConfig = toml::from_str(&test_str).unwrap();
        assert_eq!(redis.chunk_size, 100);
        assert_eq!(redis.blocktime_millis, 1000);
        assert_eq!(redis.consumer_group, "log-ingestor");
        assert_eq!(redis.consumer_id, "log-ingestor");
    }

    #[test]
    fn test_invalid_urlport_missing_field() {
        let test_str = "
url = \"http://localhost\"
";
        let result: Result<UrlPort, _> = toml::from_str(&test_str);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_toml() {
        let test_str = "
this is not toml
";
        let result: Result<RedisConfig, _> = toml::from_str(&test_str);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_file_error() {
        use std::path::PathBuf;
        let path = PathBuf::from("/nonexistent/config.toml");
        let result = std::panic::catch_unwind(|| {
            IngestorConfig::from_file(path);
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_from_file_success() {
        use std::path::PathBuf;
        let path = PathBuf::from("./install/example_config.toml");
        let config = IngestorConfig::from_file(path);
        assert_eq!(config.loki.auth.password, "test-loki-password");
        assert_eq!(config.loki.auth.username, "test-loki");
    }

    #[test]
    fn test_assembled_from_file_success() {
        use std::path::PathBuf;
        let path = PathBuf::from("./install/example_config.toml");
        let metrics_path = PathBuf::from("./install/example_metrics_config.toml");
        let config = assemble_config((path, Some(metrics_path)));
        assert_eq!(config.loki.auth.password, "test-loki-password");
        assert_eq!(config.loki.auth.username, "test-loki");
        assert_eq!(config.metrics.intervals.len(), 5);
    }

    #[test]
    fn test_assembled_from_file_preference_order() {
        use std::path::PathBuf;
        let path = PathBuf::from("./install/example_config.toml");
        let metrics_path = PathBuf::from("./install/example_metrics_config.toml");
        let config = assemble_config((path, Some(metrics_path)));
        assert_eq!(config.loki.auth.password, "test-loki-password");
        assert_eq!(config.loki.auth.username, "test-loki");
        assert_eq!(
            config.metrics.intervals.get("cpu_usage_percent"),
            Some(&MetricInterval::Secondly(30)) // from commandline arg
        );
        assert_eq!(
            config.metrics.intervals.get("ram_usage_bytes"),
            Some(&MetricInterval::Secondly(15)) // from referenced file
        );
        assert_eq!(
            config.metrics.intervals.get("new_metric"),
            Some(&MetricInterval::Secondly(53)) // from direct config file
        );
    }

    #[test]
    fn test_interval_parse() {
        let test_str = "
url = \"http://localhost\"

[intervals]
metric_1 = { Weekly = 1 }
metric_2 = { Secondly = 10 }
        ";
        let intervals: MetricIntervalsConfig = toml::from_str(&test_str).unwrap();
        println!("{intervals:?}");
        assert_eq!(
            intervals.intervals,
            HashMap::from([
                ("metric_1".into(), MetricInterval::Weekly(1)),
                ("metric_2".into(), MetricInterval::Secondly(10)),
            ])
        );
    }
}
