use crate::config::IngestorConfig;
use crate::models::{LogMessagePack, LogMsg, error_log_item};
use redis::Commands;
use std::{thread, time::Duration};
use tokio::sync::mpsc;
use tokio::time::sleep;

const LOGGING_ENDPOINT: [&str; 1] = ["info/log"];
const KEY_MISMATCH: &str = "We got a response for request with one key, there must be one key!";
const NO_DATA: &str = "Uh oh, log message contained no data";
const BAD_DATA: &str = "Log message data not binary-data or could not be decoded!";

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum RedisError {
    #[error("Temporary error: {0}")]
    Retryable(String),
    #[error("Fatal error: {}. Context: {}", 0.0, 0.1)]
    Fatal((String, Box<Option<RedisError>>)),
}

fn retryable_code(e: redis::RedisError) -> RedisError {
    RedisError::Retryable(format!("Code: {}", e.code().unwrap_or("unknown")))
}
fn fatal_code(e: redis::RedisError) -> RedisError {
    RedisError::Fatal((
        format!("Code: {}", e.code().unwrap_or("unknown")),
        Box::new(None),
    ))
}

pub fn create_redis_conn_with_retry(
    config: &'static IngestorConfig,
    max_retries: u8,
    initial_sleep: u64,
) -> Result<redis::Connection, RedisError> {
    let mut retries: u8 = 0;
    let mut last_error: Option<RedisError> = None;
    while retries < max_retries {
        match redis::Client::open(config.redis.url.full_url()) {
            Ok(c) => match c.get_connection() {
                Ok(mut c) => {
                    println!("INFO: Reconnected to redis, checking logging keys and groups...");
                    match check_connection(&mut c, config) {
                        Ok(()) => return Ok(c),
                        Err(e) => last_error = Some(e),
                    }
                }
                Err(e) => last_error = Some(retryable_code(e)),
            },
            Err(e) => last_error = Some(retryable_code(e)),
        }
        let sleep_time = initial_sleep * (2_i32.pow(retries.into()) as u64);
        println!("ERROR: {last_error:?}, retrying connection in {sleep_time} ms");
        thread::sleep(Duration::from_millis(sleep_time));
        retries += 1;
    }
    Err(RedisError::Fatal((
        "Max retries exceeded".into(),
        Box::new(last_error),
    )))
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
) -> Result<(Option<String>, Vec<redis::Value>), RedisError> {
    let raw_reply: redis::streams::StreamReadReply = redis_conn
        .xread_options(&LOGGING_ENDPOINT, &[last_id], &stream_read_opts(config))
        .map_err(retryable_code)?;

    if raw_reply.keys.is_empty() {
        return Ok((Some(last_id.to_owned()), vec![]));
    }

    let log_key = raw_reply
        .keys
        .first()
        .ok_or_else(|| RedisError::Retryable(KEY_MISMATCH.into()))?;

    let last_id = log_key.ids.last().map(|i| i.id.clone());
    let logs = log_key
        .ids
        .iter()
        .map(|e| {
            e.map
                .get("data")
                .ok_or_else(|| RedisError::Retryable(NO_DATA.into()))
                .cloned()
        })
        .collect::<Result<Vec<redis::Value>, RedisError>>()?;

    Ok((last_id, logs))
}

fn process_data(values: Vec<redis::Value>) -> Result<Vec<LogMessagePack>, RedisError> {
    let un_valued: Vec<Vec<u8>> = values
        .iter()
        .map(|e| match e {
            redis::Value::BulkString(x) => Ok(x.to_vec()),
            _ => Err(RedisError::Retryable(BAD_DATA.into())),
        })
        .collect::<Result<Vec<Vec<u8>>, RedisError>>()?;

    un_valued
        .iter()
        .map(|e| rmp_serde::from_slice::<LogMessagePack>(e))
        .collect::<Result<Vec<LogMessagePack>, rmp_serde::decode::Error>>()
        .map_err(|_| RedisError::Retryable(BAD_DATA.into()))
}

fn extract_records(messages: Vec<LogMessagePack>) -> Vec<LogMsg> {
    messages
        .iter()
        .map(|e| e.bec_codec.data.log_msg.clone())
        .collect()
}

fn setup_consumer_group(
    conn: &mut redis::Connection,
    config: &'static IngestorConfig,
) -> Result<(), RedisError> {
    println!("INFO: Setting up consumer group");
    match conn.xgroup_create::<_, _, _, ()>(&LOGGING_ENDPOINT, &config.redis.consumer_group, "0") {
        Ok(_) => {
            println!("INFO: Done setting up consumer group");
            Ok(())
        }
        Err(error) => {
            if let Some(code) = error.code()
                && code == "BUSYGROUP"
            {
                println!(
                    "INFO: Group {} already exists, rejoining with ID {}",
                    &config.redis.consumer_group, &config.redis.consumer_id
                );
                Ok(())
            } else {
                Err(RedisError::Fatal((
                    format!(
                        "Failed to create Redis consumer group {}! Code: {:?}",
                        &config.redis.consumer_group,
                        &error.code()
                    ),
                    Box::new(None),
                )))
            }
        }
    }
}

fn check_connection(
    redis_conn: &mut redis::Connection,
    config: &'static IngestorConfig,
) -> Result<(), RedisError> {
    let key_exists = redis_conn
        .exists::<&str, bool>(LOGGING_ENDPOINT[0])
        .map_err(fatal_code)?;
    if !key_exists {
        Err(RedisError::Retryable("No logging endpoint found".into()))
    } else {
        setup_consumer_group(redis_conn, config)
    }
}

pub async fn producer_loop(
    tx: mpsc::UnboundedSender<LogMsg>,
    config: &'static IngestorConfig,
    max_retries: u8,
    initial_sleep: u64,
) -> Result<(), RedisError> {
    let mut redis_conn = create_redis_conn_with_retry(config, max_retries, initial_sleep)?;
    let stream_read_id: String = ">".into();
    println!("DEBUG: Starting Loki task producer loop");
    'main: loop {
        // Sleep between blocking calls prevents starvation of other tasks in thread limited environments
        sleep(Duration::from_millis(10)).await;
        match read_logs(&mut redis_conn, &stream_read_id, config) {
            Ok(response) => {
                if let (Some(_), packed) = response {
                    if packed.is_empty() {
                        continue;
                    }
                    for record in
                        extract_records(process_data(packed).unwrap_or(vec![error_log_item()]))
                    {
                        if tx.send(record).is_err() {
                            println!("INFO: Receiver dropped, stopping...");
                            break 'main Ok(());
                        }
                    }
                }
            }
            Err(e) => {
                println!("ERROR: {:?}", e);
                redis_conn = create_redis_conn_with_retry(config, max_retries, initial_sleep)?;
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
