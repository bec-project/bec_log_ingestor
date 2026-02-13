use crate::config::IngestorConfig;
use crate::models::{LogMessagePack, LogMsg, error_log_item};
use redis::Commands;
use rmp_serde;
use std::{error::Error, thread, time::Duration};
use tokio::sync::mpsc;

const LOGGING_ENDPOINT: [&str; 1] = ["info/log"];
const KEY_MISMATCH: &str = "We got a response for request with one key, there must be one key!";
const NO_DATA: &str = "Uh oh, log message contained no data";

fn str_error(err: &str) -> Box<dyn Error> {
    Box::<dyn Error>::from(err)
}

pub fn create_redis_conn(url: &str) -> Result<redis::Connection, redis::RedisError> {
    let client = redis::Client::open(url)?;
    client.get_connection()
}

fn stream_read_opts(config: &'static IngestorConfig) -> redis::streams::StreamReadOptions {
    redis::streams::StreamReadOptions::default()
        .count(config.redis.chunk_size.into())
        .block(config.redis.blocktime_millis)
        .group("log-ingestor", "log-ingestor")
}

/// Fetch unread logs for redis.
/// Returns a tuple of the last ID read and a Vec of msgpacked entries from the log stream endpoint
fn read_logs(
    redis_conn: &mut redis::Connection,
    last_id: &String,
    config: &'static IngestorConfig,
) -> Result<(Option<String>, Vec<redis::Value>), Box<dyn Error>> {
    let raw_reply: redis::streams::StreamReadReply =
        redis_conn.xread_options(&LOGGING_ENDPOINT, &[last_id], &stream_read_opts(config))?;

    if raw_reply.keys.is_empty() {
        return Ok((Some(last_id.to_owned()), vec![]));
    }

    let log_key = raw_reply
        .keys
        .first()
        .ok_or_else(|| str_error(KEY_MISMATCH))?;

    let last_id = log_key.ids.last().map(|i| i.id.clone());
    let logs = log_key
        .ids
        .iter()
        .map(|e| e.map.get("data").ok_or_else(|| str_error(NO_DATA)).cloned())
        .collect::<Result<Vec<redis::Value>, Box<dyn Error>>>()?;

    Ok((last_id, logs))
}

fn process_data(values: Vec<redis::Value>) -> Result<Vec<LogMessagePack>, Box<dyn Error>> {
    let un_valued: Vec<Vec<u8>> = values
        .iter()
        .map(|e| match e {
            redis::Value::BulkString(x) => Ok(x.to_vec()),
            _ => Err(str_error("Log message data not binary-data!")),
        })
        .collect::<Result<Vec<Vec<u8>>, Box<dyn Error>>>()?;

    Ok(un_valued
        .iter()
        .map(|e| rmp_serde::from_slice::<LogMessagePack>(e))
        .collect::<Result<Vec<LogMessagePack>, rmp_serde::decode::Error>>()?)
}

fn extract_records(messages: Vec<LogMessagePack>) -> Vec<LogMsg> {
    messages
        .iter()
        .map(|e| e.bec_codec.data.log_msg.clone())
        .collect()
}

fn setup_consumer_group(conn: &mut redis::Connection, config: &'static IngestorConfig) {
    let group: Result<(), redis::RedisError> =
        conn.xgroup_create(&LOGGING_ENDPOINT, &config.redis.consumer_group, "0");
    match group {
        Ok(_) => (),
        Err(error) => {
            if let Some(code) = error.code()
                && code == "BUSYGROUP"
            {
                println!(
                    "Group {} already exists, rejoining with ID {}",
                    &config.redis.consumer_group, &config.redis.consumer_id
                )
            } else {
                panic!(
                    "Failed to create Redis consumer group {}! Code: {:?}",
                    &config.redis.consumer_group,
                    &error.code()
                );
            }
        }
    }
    let create_id: Result<(), redis::RedisError> = conn.xgroup_createconsumer(
        &LOGGING_ENDPOINT,
        &config.redis.consumer_group,
        &config.redis.consumer_id,
    );
    create_id.expect(&format!(
        "Failed to create Redis consumer ID {} in group {}!",
        &config.redis.consumer_id, &config.redis.consumer_group
    ));
}

fn check_connection(
    redis_conn: &mut redis::Connection,
    config: &'static IngestorConfig,
) -> Result<(), ()> {
    if let Ok(key_exists) = redis_conn.exists::<&str, bool>(&LOGGING_ENDPOINT[0]) {
        if !key_exists {
            println!("Logging endpoint doesn't exist, exiting.");
            return Err(());
        }
    } else {
        panic!("Something went wrong checking the logs endpoint")
    }
    setup_consumer_group(redis_conn, &config);
    Ok(())
}
pub async fn producer_loop(tx: mpsc::UnboundedSender<LogMsg>, config: &'static IngestorConfig) {
    let mut redis_conn =
        create_redis_conn(&config.redis.url.full_url()).expect("Could not connect to Redis!");
    if check_connection(&mut redis_conn, &config).is_err() {
        println!("");
        return;
    }

    let stream_read_id: String = ">".into();

    'main: loop {
        let raw_read = read_logs(&mut redis_conn, &stream_read_id, &config);
        if let Ok(response) = raw_read {
            if let (Some(_), packed) = response {
                if packed.is_empty() {
                    continue;
                }
                let unpacked = process_data(packed).unwrap_or(vec![error_log_item()]);
                let records = extract_records(unpacked);

                for record in records {
                    if tx.send(record).is_err() {
                        println!("Receiver dropped, stopping...");
                        break 'main;
                    }
                }
            }
        } else {
            println!("{:?}", raw_read);
            redis_conn = loop {
                {
                    let new_conn = create_redis_conn(&config.redis.url.full_url());
                    if new_conn.is_err() {
                        println!("Error reading from redis, retrying connection in 1s");
                        thread::sleep(Duration::from_millis(1000));
                    } else {
                        println!("Reconnected");
                        let mut conn = new_conn.unwrap();
                        if check_connection(&mut conn, &config).is_err() {
                            return;
                        }
                        break conn;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::models::LogRecord;

    use super::*;

    #[test]
    fn test_error_log_item_contents() {
        let err_item = error_log_item();
        assert_eq!(err_item.bec_codec.data.log_msg.record.level.name, "ERROR");
        assert_eq!(
            err_item.bec_codec.data.log_msg.record.message,
            "Error processing log messages from Redis!"
        );
    }

    #[test]
    fn test_extract_records() {
        let mut pack = error_log_item();
        pack.bec_codec.data.log_msg.record.message = "test".to_string();
        let records = extract_records(vec![pack.clone()]);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].record.message, "test");
    }

    #[test]
    fn test_process_data_valid() {
        let pack = error_log_item();
        let bytes = rmp_serde::to_vec(&pack).unwrap();
        let redis_val = redis::Value::BulkString(bytes.into());
        let result = process_data(vec![redis_val]);
        assert!(result.is_ok());
        let unpacked = result.unwrap();
        assert_eq!(unpacked.len(), 1);
        assert_eq!(
            unpacked[0].bec_codec.data.log_msg.record.level.name,
            "ERROR"
        );
    }

    #[test]
    fn test_process_data_invalid_type() {
        let redis_val = redis::Value::Int(42);
        let result = process_data(vec![redis_val]);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_records_empty() {
        let records = extract_records(vec![]);
        assert!(records.is_empty());
    }

    #[test]
    fn test_logrecord_serde_roundtrip() {
        let record = error_log_item().bec_codec.data.log_msg.record.clone();
        let ser = serde_json::to_string(&record).unwrap();
        let de: LogRecord = serde_json::from_str(&ser).unwrap();
        assert_eq!(record, de);
    }
}
