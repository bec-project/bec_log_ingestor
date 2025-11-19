use std::{collections::HashMap, fmt, io::Read};

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
        toml::from_str(&contents).unwrap()
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
    Secondly(i32),
    Minutely(i32),
    Hourly(i32),
    Daily(i32),
    Weekly(i32),
}
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct MetricsConfig {
    url: String,
    #[serde(default = "HashMap::new")]
    intervals: HashMap<String, MetricInterval>,
}
impl FromTomlFile for MetricsConfig {}

#[derive(Clone, Debug, Deserialize)]
pub struct IngestorConfig {
    pub redis: RedisConfig,
    pub loki: LokiConfig,
}
impl FromTomlFile for IngestorConfig {}

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
    fn test_interval_parse() {
        let test_str = "
url = \"http://localhost\"

[intervals]
metric_1 = { Weekly = 1 }
metric_2 = { Secondly = 10 }
        ";
        let intervals: MetricsConfig = toml::from_str(&test_str).unwrap();
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
