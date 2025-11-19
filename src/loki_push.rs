use tokio::sync::mpsc;

use std::{error::Error, iter::once};

use crate::{config::LokiConfig, models::LogMsg};

/// Convert a LogRecord to the document we want Loki to ingest
fn json_from_logmsg(
    msg: &LogMsg,
    config: &LokiConfig,
) -> Result<serde_json::Value, serde_json::Error> {
    Ok(serde_json::json!([
            msg.record.time.as_epoch_nanos(),
            msg.record.line,
            {
                "file_name": msg.record.file.name,
                "file_location": msg.record.file.path,
                "function": msg.record.function,
                "message": msg.record.message,
                "log_type": msg.record.level.name,
                "module": msg.record.module,
                "service_name": msg.service_name,
                "beamline_name": config.beamline_name,
                "proc_id": msg.record.process.id,
                "exception": msg.record.exception,
            }
        ]
    ))
}

fn make_json_body(
    msgs: &Vec<LogMsg>,
    config: &LokiConfig,
) -> Result<serde_json::Value, serde_json::Error> {
    let values = msgs
        .iter()
        .map(|e| json_from_logmsg(e, config))
        .collect::<Result<Vec<serde_json::Value>, serde_json::Error>>()?;

    Ok(serde_json::json!({
        "streams" : [
            {
                "stream" :{
                    "label": "bec_logs"
                },
                "values": values
            }
        ]
    }))
}

pub async fn consumer_loop(rx: &mut mpsc::UnboundedReceiver<LogMsg>, config: LokiConfig) {

    // let mut buffer: Vec<LogMsg> = Vec::with_capacity(config.chunk_size.into());

    // loop {
    //     let open = rx.recv_many(&mut buffer, config.chunk_size.into()).await;
    //     if open == 0 {
    //         break;
    //     }
    //     let body = make_json_body(&buffer, &config).unwrap_or(vec![]);
    //     let response = elastic_client
    //         .bulk(elasticsearch::BulkParts::Index(&config.index))
    //         .body(body)
    //         .send()
    //         .await;
    //     println!(
    //         "sent {} logs to elastic, response OK: {:?}",
    //         open,
    //         response.is_ok()
    //     );
    //     buffer.clear();
    // }
    // println!("Producer dropped, consumer exiting");
}

#[cfg(test)]
mod tests {
    use crate::{config::UrlPort, models::LogRecord};

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

    fn loki_config() -> LokiConfig {
        let test_str = "
url = { url = \"http://localhost\", port = 9200 }
api_key = \"testkey\"
";
        toml::from_str(&test_str).unwrap()
    }

    #[test]
    fn test_make_docs_values_empty() {
        let records: Vec<LogMsg> = vec![];
        let docs = make_json_body(&records, &loki_config()).unwrap();
        assert_eq!(
            docs.to_string(),
            "{\"streams\":[{\"stream\":{\"label\":\"bec_logs\"},\"values\":[]}]}"
        );
    }

    #[test]
    fn test_make_docs_values_single() {
        let record: LogMsg = DummyLog {
            msg: "hello".to_string(),
            level: "info".to_string(),
        }
        .into();
        let docs = make_json_body(&vec![record.clone()], &loki_config()).unwrap();
        // Each record should produce two JSON bodies (action + doc)
        assert_eq!(
            docs.to_string(),
            "{\"streams\":[{\"stream\":{\"label\":\"bec_logs\"},\"values\":[[\"0\",0,{\"beamline_name\":\"x99xa\",\"exception\":null,\"file_location\":\"\",\"file_name\":\"\",\"function\":\"\",\"log_type\":\"info\",\"message\":\"hello\",\"module\":\"\",\"proc_id\":0,\"service_name\":\"test_service\"}]]}]}"
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
        let docs = make_json_body(&vec![record1, record2], &loki_config()).unwrap();
        assert_eq!(
            docs.to_string(),
            "{\"streams\":[{\"stream\":{\"label\":\"bec_logs\"},\"values\":[[\"0\",0,{\"beamline_name\":\"x99xa\",\"exception\":null,\"file_location\":\"\",\"file_name\":\"\",\"function\":\"\",\"log_type\":\"info\",\"message\":\"a\",\"module\":\"\",\"proc_id\":0,\"service_name\":\"test_service\"}],[\"0\",0,{\"beamline_name\":\"x99xa\",\"exception\":null,\"file_location\":\"\",\"file_name\":\"\",\"function\":\"\",\"log_type\":\"warn\",\"message\":\"b\",\"module\":\"\",\"proc_id\":0,\"service_name\":\"test_service\"}]]}]}"
        );
    }
}
