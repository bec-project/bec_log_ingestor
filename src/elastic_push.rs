use elasticsearch::{Elasticsearch, http::request::JsonBody};
use tokio::sync::mpsc;

use std::{error::Error, iter::once};

use crate::{config::ElasticConfig, redis_logs::LogMsg};

fn elastic_client(config: &ElasticConfig) -> Result<Elasticsearch, Box<dyn Error>> {
    let url = elasticsearch::http::Url::parse(&config.url.full_url())?;
    let conn_pool = elasticsearch::http::transport::SingleNodeConnectionPool::new(url);
    let credentials = config.credentials()?;
    let transport = elasticsearch::http::transport::TransportBuilder::new(conn_pool)
        .auth(credentials)
        .cert_validation(elasticsearch::cert::CertificateValidation::None)
        .build()?;
    Ok(Elasticsearch::new(transport))
}

/// Convert a LogRecord to the document we want Elastic to ingest
fn json_from_logmsg(msg: &LogMsg) -> Result<serde_json::Value, serde_json::Error> {
    // dbg!(serde_json::to_value(record))
    dbg!(Ok(serde_json::json!({
        "@timestamp": msg.record.time.as_rfc3339(),
        "file": msg.record.file,
        "function": msg.record.function,
        "message": msg.record.message,
        "log_type": msg.record.level.name,
        "line": msg.record.line,
        "module": msg.record.module,
        "service_name": msg.service_name,
        "proc_id": msg.record.process.id,
        "exception": msg.record.exception,
    })))
}

fn make_json_body(
    msgs: &Vec<LogMsg>,
) -> Result<Vec<JsonBody<serde_json::Value>>, serde_json::Error> {
    let action = serde_json::json!({ "create": {} });

    let values = msgs
        .iter()
        .map(|e| json_from_logmsg(e))
        .collect::<Result<Vec<serde_json::Value>, serde_json::Error>>()?;

    Ok(values
        .iter()
        .flat_map(|e| {
            once(JsonBody::from(action.clone())).chain(once(JsonBody::from(e.to_owned())))
        })
        .collect())
}

pub async fn consumer_loop(rx: &mut mpsc::UnboundedReceiver<LogMsg>, config: ElasticConfig) {
    let elastic_client = elastic_client(&config).expect("Failed to connect to Elastic!");

    let mut buffer: Vec<LogMsg> = Vec::with_capacity(config.chunk_size.into());

    loop {
        let open = rx.recv_many(&mut buffer, config.chunk_size.into()).await;
        if open == 0 {
            break;
        }
        let body = make_json_body(&buffer).unwrap_or(vec![]);
        let response = elastic_client
            .bulk(elasticsearch::BulkParts::Index(&config.index))
            .body(body)
            .send()
            .await;
        println!(
            "sent {} logs to elastic, response OK: {:?} \n {:?}",
            open,
            response.is_ok(),
            response
        );
        buffer.clear();
    }
    println!("Producer dropped, consumer exiting");
}

#[cfg(test)]
mod tests {
    use crate::{config::UrlPort, redis_logs::LogRecord};

    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct DummyLog {
        msg: String,
        level: String,
    }

    // Implement LogRecord for test if not already present
    impl From<DummyLog> for LogMsg {
        fn from(d: DummyLog) -> Self {
            // Adjust this conversion as per your actual LogRecord struct
            LogMsg {
                service_name: "test_service".into(),
                text: "...".into(),
                record: LogRecord {
                    elapsed: crate::redis_logs::Elapsed {
                        repr: "".into(),
                        seconds: 0.0,
                    },
                    exception: None,
                    extra: {}.into(),
                    file: crate::redis_logs::File {
                        name: "".into(),
                        path: "".into(),
                    },
                    function: "".into(),
                    level: crate::redis_logs::LogLevel {
                        icon: "".into(),
                        name: d.level,
                        no: 100,
                    },
                    line: 0,
                    message: d.msg,
                    module: "".into(),
                    name: "".into(),
                    process: crate::redis_logs::NameId {
                        name: "".into(),
                        id: 0,
                    },
                    thread: crate::redis_logs::NameId {
                        name: "".into(),
                        id: 0,
                    },
                    time: crate::redis_logs::Timestamp {
                        repr: "".into(),
                        timestamp: 0.0,
                    },
                },
            }
        }
    }

    #[test]
    fn test_make_docs_values_empty() {
        let records: Vec<LogMsg> = vec![];
        let docs = make_json_body(&records).unwrap();
        assert!(docs.is_empty());
    }

    #[test]
    fn test_make_docs_values_single() {
        let record: LogMsg = DummyLog {
            msg: "hello".to_string(),
            level: "info".to_string(),
        }
        .into();
        let docs = make_json_body(&vec![record.clone()]).unwrap();
        // Each record should produce two JSON bodies (action + doc)
        assert_eq!(docs.len(), 2);
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
        let docs = make_json_body(&vec![record1, record2]).unwrap();
        assert_eq!(docs.len(), 4);
    }

    #[test]
    fn test_elastic_client_invalid_url() {
        let result = elastic_client(&ElasticConfig {
            url: UrlPort {
                url: "not an url".into(),
                port: 9876,
            },
            api_key: Some("key".into()),
            username: None,
            password: None,
            chunk_size: 8,
            index: "".into(),
        });
        assert!(result.is_err());
    }
}
