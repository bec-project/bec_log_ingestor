use std::process::exit;
use std::{collections::BTreeMap, time::Duration};

use tokio::{
    sync::mpsc,
    time::{Interval, sleep},
};

use crate::{
    config::{IngestorConfig, LokiConfig},
    models::{AckAction, LogMsg, RedisLogBatch},
};

fn is_timestamp_too_old_response(text: &str) -> bool {
    text.contains("timestamp too old")
}

fn should_retry_loki_response(status: reqwest::StatusCode, text: &str) -> bool {
    if status.is_server_error() {
        return !text.contains("unmarshalerDecoder: Value is string");
    }
    status.as_u16() == 429
}

fn dropped_logs_summary(records: &[LogMsg]) -> String {
    const MAX_LOGS_TO_PRINT: usize = 5;
    const MAX_MESSAGE_CHARS: usize = 200;

    let mut summary = records
        .iter()
        .take(MAX_LOGS_TO_PRINT)
        .map(|record| {
            let mut message: String = record.record.message.chars().take(MAX_MESSAGE_CHARS).collect();
            if record.record.message.chars().count() > MAX_MESSAGE_CHARS {
                message.push_str("...");
            }
            format!(
                "[{}] {}: {}",
                record.record.level.name, record.service_name, message
            )
        })
        .collect::<Vec<_>>()
        .join(" | ");

    if records.len() > MAX_LOGS_TO_PRINT {
        summary.push_str(&format!(
            " | ... {} more dropped log(s)",
            records.len() - MAX_LOGS_TO_PRINT
        ));
    }

    summary
}

fn retimestamp_logs_to_now(records: &mut [LogMsg]) {
    let now = chrono::Utc::now().timestamp_millis() as f64 / 1000.0;
    for record in records {
        record.record.time.timestamp = now;
    }
}

/// Convert a LogRecord to the document we want Loki to ingest
fn json_from_logmsg(msg: &LogMsg, config: &LokiConfig) -> (serde_json::Value, (String, String)) {
    (
        serde_json::json!([
            msg.record.time.as_epoch_nanos(),
            msg.record.message,
            {
                "file_name": msg.record.file.name,
                "file_location": msg.record.file.path,
                "function": msg.record.function,
                "line": msg.record.line.to_string(),
                "module": msg.record.module,
                "beamline_name": config.beamline_name,
                "proc_id": msg.record.process.id.to_string(),
                "exception": msg.record.exception.clone().unwrap_or("None".into())
            }
        ]),
        (
            msg.record.level.name.to_owned(),
            msg.service_name.to_owned(),
        ),
    )
}

fn make_json_body(msgs: &[LogMsg], config: &'static IngestorConfig) -> serde_json::Value {
    let values = msgs
        .iter()
        .map(|e| json_from_logmsg(e, &config.loki))
        .collect::<Vec<(serde_json::Value, (String, String))>>();

    let mut map: BTreeMap<(String, String), Vec<serde_json::Value>> = BTreeMap::new();

    for (value, key_pair) in values {
        map.entry(key_pair).or_default().push(value);
    }

    let streams: Vec<serde_json::Value> = map
        .iter()
        .map(|(k, v)| {
            serde_json::json!({
                "stream" :{
                    "label": "bec_logs",
                    "hostname": &config.hostname,
                    "level": k.0,
                    "service_name":k.1
                },
                "values": v
            })
        })
        .collect();

    serde_json::json!({
        "streams": streams
    })
}

pub async fn consumer_loop(
    rx: &mut mpsc::UnboundedReceiver<RedisLogBatch>,
    ack_tx: mpsc::UnboundedSender<AckAction>,
    config: &'static IngestorConfig,
) {
    let mut buffer: Vec<RedisLogBatch> = Vec::with_capacity(config.loki.chunk_size.into());
    let mut records: Vec<LogMsg> = Vec::with_capacity(config.loki.chunk_size.into());
    let mut ack_ids: Vec<String> = Vec::with_capacity(config.loki.chunk_size.into());
    let mut body = String::new();
    let mut retries: u8 = 0;
    let mut stop_requested = false;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to build Loki HTTP client");
    let mut interval: Interval = (&config.loki.push_interval).into();
    println!(
        "INFO: Starting Loki consumer loop, pushing at {:?}.",
        &config.loki.push_interval
    );

    loop {
        interval.tick().await;
        if buffer.is_empty() {
            let open = rx
                .recv_many(&mut buffer, config.loki.chunk_size.into())
                .await;
            if open == 0 {
                break;
            }
        };
        if body.is_empty() {
            records.clear();
            ack_ids.clear();
            for batch in &buffer {
                ack_ids.extend(batch.entry_ids.iter().cloned());
                records.extend(batch.records.iter().cloned());
            }
            body = make_json_body(&records, config).to_string();
        }
        let pending_messages = records.len();
        println!("DEBUG: {pending_messages} log messages waiting to be pushed to Loki.");
        match client
            .post(&config.loki.url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .basic_auth(&config.loki.auth.username, Some(&config.loki.auth.password))
            .body(body.clone())
            .send()
            .await
        {
            Ok(res) => {
                if !res.status().is_success() {
                    let status = res.status();
                    let text = res
                        .text()
                        .await
                        .unwrap_or("[Unable to decode response text!]".into());
                    if is_timestamp_too_old_response(&text) {
                        println!(
                            "WARNING: Retimestamping {pending_messages} buffered logs to now because Loki rejected them as too old: {text}"
                        );
                        retries = 0;
                        retimestamp_logs_to_now(&mut records);
                        body = make_json_body(&records, config).to_string();
                        continue;
                    }
                    println!("ERROR: Received error response from Loki: {text}");
                    if !should_retry_loki_response(status, &text) {
                        println!(
                            "WARNING: Acknowledging {pending_messages} buffered logs and stopping because the response is not retryable."
                        );
                        println!("WARNING: Dropped log contents: {}", dropped_logs_summary(&records));
                        if ack_tx.send(AckAction::AckAndStop(ack_ids.clone())).is_err() {
                            println!("ERROR: Failed to send ack IDs back to Redis producer, exiting.");
                            exit(69);
                        }
                        buffer.clear();
                        records.clear();
                        ack_ids.clear();
                        body.clear();
                        stop_requested = true;
                        break;
                    }
                    retries = retries.saturating_add(1);
                    if retries >= 3 {
                        println!("ERROR: Maximum Loki retry attempts exceeded, exiting.");
                        exit(69);
                    }
                    println!("WARNING: Retrying buffered logs in 5s.");
                    sleep(Duration::from_secs(5)).await;
                    continue;
                }
                retries = 0;
                println!("DEBUG: Sent {pending_messages} logs to Loki. Response: {res:?}");
                if ack_tx.send(AckAction::Ack(ack_ids.clone())).is_err() {
                    println!("ERROR: Failed to send ack IDs back to Redis producer, exiting.");
                    exit(69);
                }
            }
            Err(res) => {
                println!("ERROR: {res:?}");
                retries = retries.saturating_add(1);
                if retries >= 3 {
                    println!("ERROR: Maximum Loki retry attempts exceeded, exiting.");
                    exit(69);
                }
                println!("WARNING: Retrying buffered logs in 5s.");
                sleep(Duration::from_secs(5)).await;
                continue;
            }
        };
        buffer.clear();
        records.clear();
        ack_ids.clear();
        body.clear();
        sleep(Duration::from_millis(10)).await;
    }
    if stop_requested {
        println!("ERROR: Consumer exiting after acknowledging a poison log bundle.");
    } else {
        println!("INFO: Producer dropped, consumer exiting");
    }
}

#[cfg(test)]
mod tests {
    use crate::{config::assemble_config, models::log_message::LogRecord};

    use super::*;

    use serde_derive::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct DummyLog {
        msg: String,
        level: String,
    }

    impl From<DummyLog> for LogMsg {
        fn from(d: DummyLog) -> Self {
            LogMsg {
                service_name: "test_service".into(),
                text: "...".into(),
                record: LogRecord {
                    elapsed: crate::models::log_message::Elapsed {
                        repr: "".into(),
                        seconds: 0.0,
                    },
                    exception: None,
                    extra: {}.into(),
                    file: crate::models::log_message::File {
                        name: "".into(),
                        path: "".into(),
                    },
                    function: "".into(),
                    level: crate::models::log_message::LogLevel {
                        icon: "".into(),
                        name: d.level,
                        no: 100,
                    },
                    line: 0,
                    message: d.msg,
                    module: "".into(),
                    name: "".into(),
                    process: crate::models::log_message::NameId {
                        name: "".into(),
                        id: 0,
                    },
                    thread: crate::models::log_message::NameId {
                        name: "".into(),
                        id: 0,
                    },
                    time: crate::models::log_message::Timestamp {
                        repr: "".into(),
                        timestamp: 0.0,
                    },
                },
            }
        }
    }

    fn config() -> &'static IngestorConfig {
        use std::path::PathBuf;
        let path = PathBuf::from("./install/example_config.toml");
        let metrics_path = PathBuf::from("./install/example_metrics_config.toml");
        Box::leak(Box::new(assemble_config((path, Some(metrics_path)))))
    }

    #[test]
    fn test_detect_timestamp_too_old_response() {
        assert!(is_timestamp_too_old_response(
            "entry for stream '{foo=\"bar\"}' has timestamp too old: 2025-10-09T12:29:45Z"
        ));
        assert!(!is_timestamp_too_old_response("internal server error"));
    }

    #[test]
    fn test_should_retry_loki_response() {
        assert!(should_retry_loki_response(
            reqwest::StatusCode::INTERNAL_SERVER_ERROR,
            "internal server error"
        ));
        assert!(!should_retry_loki_response(
            reqwest::StatusCode::BAD_REQUEST,
            "loghttp.PushRequest.Streams: []loghttp.LogProtoStream: unmarshalerDecoder: Value is string"
        ));
        assert!(should_retry_loki_response(
            reqwest::StatusCode::TOO_MANY_REQUESTS,
            "rate limited"
        ));
    }

    #[test]
    fn test_dropped_logs_summary() {
        let record1: LogMsg = DummyLog {
            msg: "hello".to_string(),
            level: "info".to_string(),
        }
        .into();
        let record2: LogMsg = DummyLog {
            msg: "world".to_string(),
            level: "warn".to_string(),
        }
        .into();

        assert_eq!(
            dropped_logs_summary(&[record1, record2]),
            "[info] test_service: hello | [warn] test_service: world"
        );
    }

    #[test]
    fn test_retimestamp_logs_to_now() {
        let mut record: LogMsg = DummyLog {
            msg: "hello".to_string(),
            level: "info".to_string(),
        }
        .into();
        record.record.time.timestamp = 1.0;
        let mut records = vec![record];

        retimestamp_logs_to_now(&mut records);

        assert!(records[0].record.time.timestamp > 1.0);
    }

    #[test]
    fn test_make_docs_values_empty() {
        let records: Vec<LogMsg> = vec![];
        let docs = make_json_body(&records, config());
        assert_eq!(docs.to_string(), "{\"streams\":[]}");
    }

    #[test]
    fn test_make_docs_values_single() {
        let record: LogMsg = DummyLog {
            msg: "hello".to_string(),
            level: "info".to_string(),
        }
        .into();
        let docs = make_json_body(&vec![record.clone()], config());
        // Each record should produce two JSON bodies (action + doc)
        let hostname = gethostname::gethostname()
            .into_string()
            .unwrap_or("failed_to_parse_hostname".into());
        assert_eq!(
            docs.to_string(),
            format!(
                "{{\"streams\":[{{\"stream\":{{\"hostname\":\"{}\",\"label\":\"bec_logs\",\"level\":\"info\",\"service_name\":\"test_service\"}},\"values\":[[\"0\",\"hello\",{{\"beamline_name\":\"x99xa\",\"exception\":\"None\",\"file_location\":\"\",\"file_name\":\"\",\"function\":\"\",\"line\":\"0\",\"module\":\"\",\"proc_id\":\"0\"}}]]}}]}}",
                hostname
            )
        );
    }

    #[test]
    fn test_make_docs_values_multiple() {
        let record1: LogMsg = DummyLog {
            msg: "a".to_string(),
            level: "info".to_string(),
        }
        .into();
        let record2: LogMsg = DummyLog {
            msg: "b".to_string(),
            level: "warn".to_string(),
        }
        .into();
        let docs = make_json_body(&vec![record1, record2], config());
        let hostname = gethostname::gethostname()
            .into_string()
            .unwrap_or("failed_to_parse_hostname".into());
        assert_eq!(
            docs.to_string(),
            format!(
                "{{\"streams\":[{{\"stream\":{{\"hostname\":\"{}\",\"label\":\"bec_logs\",\"level\":\"info\",\"service_name\":\"test_service\"}},\"values\":[[\"0\",\"a\",{{\"beamline_name\":\"x99xa\",\"exception\":\"None\",\"file_location\":\"\",\"file_name\":\"\",\"function\":\"\",\"line\":\"0\",\"module\":\"\",\"proc_id\":\"0\"}}]]}},{{\"stream\":{{\"hostname\":\"{}\",\"label\":\"bec_logs\",\"level\":\"warn\",\"service_name\":\"test_service\"}},\"values\":[[\"0\",\"b\",{{\"beamline_name\":\"x99xa\",\"exception\":\"None\",\"file_location\":\"\",\"file_name\":\"\",\"function\":\"\",\"line\":\"0\",\"module\":\"\",\"proc_id\":\"0\"}}]]}}]}}",
                &hostname, &hostname
            )
        );
    }
}
