use tokio::{sync::mpsc, time::Interval};

use crate::{
    config::{IngestorConfig, LokiConfig},
    models::LogMsg,
};

/// Convert a LogRecord to the document we want Loki to ingest
fn json_from_logmsg(msg: &LogMsg, config: &LokiConfig) -> serde_json::Value {
    serde_json::json!([
        msg.record.time.as_epoch_nanos(),
        msg.record.message,
        {
            "file_name": msg.record.file.name,
            "file_location": msg.record.file.path,
            "function": msg.record.function,
            "line": msg.record.line.to_string(),
            "log_type": msg.record.level.name,
            "module": msg.record.module,
            "service_name": msg.service_name,
            "beamline_name": config.beamline_name,
            "proc_id": msg.record.process.id.to_string(),
            "exception": msg.record.exception.clone().unwrap_or("None".into())
        }
    ])
}

fn make_json_body(msgs: &[LogMsg], config: &'static IngestorConfig) -> serde_json::Value {
    let values = msgs
        .iter()
        .map(|e| json_from_logmsg(e, &config.loki))
        .collect::<Vec<serde_json::Value>>();

    serde_json::json!({
        "streams" : [
            {
                "stream" :{
                    "label": format!("bec_logs_{}", &config.loki.beamline_name)
                },
                "values": values
            }
        ]
    })
}

pub async fn consumer_loop(
    rx: &mut mpsc::UnboundedReceiver<LogMsg>,
    config: &'static IngestorConfig,
) {
    let mut buffer: Vec<LogMsg> = Vec::with_capacity(config.loki.chunk_size.into());
    let client = reqwest::Client::new();
    let mut interval: Interval = (&config.loki.push_interval).into();

    loop {
        interval.tick().await;
        let open = rx
            .recv_many(&mut buffer, config.loki.chunk_size.into())
            .await;
        if open == 0 {
            break;
        }
        let body = make_json_body(&buffer, config).to_string();
        match client
            .post(&config.loki.url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .basic_auth(&config.loki.auth.username, Some(&config.loki.auth.password))
            .body(body)
            .send()
            .await
        {
            Ok(res) => {
                println!("Sent {open} logs to loki.");
                if !res.status().is_success() {
                    let text = res
                        .text()
                        .await
                        .unwrap_or("[Unable to decode response text!]".into());
                    println!("Received error response: {text} ");
                }
            }
            Err(res) => {
                println!("ERROR: {res:?}");
            }
        };
        buffer.clear();
    }
    println!("Producer dropped, consumer exiting");
}

#[cfg(test)]
mod tests {
    use crate::{config::assemble_config, models::LogRecord};

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
                    elapsed: crate::models::Elapsed {
                        repr: "".into(),
                        seconds: 0.0,
                    },
                    exception: None,
                    extra: {}.into(),
                    file: crate::models::File {
                        name: "".into(),
                        path: "".into(),
                    },
                    function: "".into(),
                    level: crate::models::LogLevel {
                        icon: "".into(),
                        name: d.level,
                        no: 100,
                    },
                    line: 0,
                    message: d.msg,
                    module: "".into(),
                    name: "".into(),
                    process: crate::models::NameId {
                        name: "".into(),
                        id: 0,
                    },
                    thread: crate::models::NameId {
                        name: "".into(),
                        id: 0,
                    },
                    time: crate::models::Timestamp {
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
    fn test_make_docs_values_empty() {
        let records: Vec<LogMsg> = vec![];
        let docs = make_json_body(&records, config());
        assert_eq!(
            docs.to_string(),
            "{\"streams\":[{\"stream\":{\"label\":\"bec_logs_x99xa\"},\"values\":[]}]}"
        );
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
        assert_eq!(
            docs.to_string(),
            "{\"streams\":[{\"stream\":{\"label\":\"bec_logs_x99xa\"},\"values\":[[\"0\",\"hello\",{\"beamline_name\":\"x99xa\",\"exception\":\"None\",\"file_location\":\"\",\"file_name\":\"\",\"function\":\"\",\"line\":\"0\",\"log_type\":\"info\",\"module\":\"\",\"proc_id\":\"0\",\"service_name\":\"test_service\"}]]}]}"
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
        assert_eq!(
            docs.to_string(),
            "{\"streams\":[{\"stream\":{\"label\":\"bec_logs_x99xa\"},\"values\":[[\"0\",\"a\",{\"beamline_name\":\"x99xa\",\"exception\":\"None\",\"file_location\":\"\",\"file_name\":\"\",\"function\":\"\",\"line\":\"0\",\"log_type\":\"info\",\"module\":\"\",\"proc_id\":\"0\",\"service_name\":\"test_service\"}],[\"0\",\"b\",{\"beamline_name\":\"x99xa\",\"exception\":\"None\",\"file_location\":\"\",\"file_name\":\"\",\"function\":\"\",\"line\":\"0\",\"log_type\":\"warn\",\"module\":\"\",\"proc_id\":\"0\",\"service_name\":\"test_service\"}]]}]}"
        );
    }
}
