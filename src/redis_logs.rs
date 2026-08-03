use crate::config::IngestorConfig;
use crate::models::{AckAction, LogMessagePack, LogMsg, RedisLogBatch, error_log_item};
use redis::Commands;
use rmp_serde::Deserializer;
use std::{thread, time::Duration};
use tokio::sync::mpsc;
use tokio::time::sleep;

const USER_LOGGING_ENDPOINT: [&str; 1] = ["user/log"];
const INFO_LOGGING_ENDPOINT: [&str; 1] = ["info/log"];
const KEY_MISMATCH: &str = "We got a response for request with one key, there must be one key!";
const NO_DATA: &str = "Uh oh, log message contained no data";
const BAD_DATA: &str = "Log message data not binary-data or could not be decoded!";

struct ReadLogsResult {
    entries: Vec<(String, redis::Value)>,
    skipped_ids: Vec<String>,
}

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

pub struct RedisConnWithKey {
    pub conn: redis::Connection,
    pub key: &'static str,
}

pub fn create_redis_conn_with_retry(
    config: &'static IngestorConfig,
    max_retries: u8,
    initial_sleep: u64,
) -> Result<RedisConnWithKey, RedisError> {
    let mut retries: u8 = 0;
    let mut last_error: Option<RedisError> = None;
    while retries < max_retries {
        match redis::Client::open(config.redis.url.full_url()) {
            Ok(c) => match c.get_connection() {
                Ok(c) => {
                    println!("INFO: Reconnected to redis, checking logging keys and groups...");
                    match check_connection(c, config) {
                        Ok(c_with_key) => return Ok(c_with_key),
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
        .group(&config.redis.consumer_group, &config.redis.consumer_id)
}

/// Fetch unread logs for redis.
/// Returns a tuple of the last ID read and a Vec of msgpacked entries from the log stream endpoint
fn read_logs(
    redis_conn: &mut RedisConnWithKey,
    last_id: &String,
    config: &'static IngestorConfig,
) -> Result<ReadLogsResult, RedisError> {
    let raw_reply: redis::streams::StreamReadReply = redis_conn
        .conn
        .xread_options(&[redis_conn.key], &[last_id], &stream_read_opts(config))
        .map_err(retryable_code)?;

    if raw_reply.keys.is_empty() {
        return Ok(ReadLogsResult {
            entries: vec![],
            skipped_ids: vec![],
        });
    }

    let log_key = raw_reply
        .keys
        .first()
        .ok_or_else(|| RedisError::Retryable(KEY_MISMATCH.into()))?;
    let mut entries = Vec::with_capacity(log_key.ids.len());
    let mut skipped_ids = Vec::new();

    for e in &log_key.ids {
        match e.map.get("data") {
            Some(data) => entries.push((e.id.clone(), data.clone())),
            None => {
                println!(
                    "WARNING: {NO_DATA}; acknowledging Redis stream entry {}",
                    e.id
                );
                skipped_ids.push(e.id.clone());
            }
        }
    }

    Ok(ReadLogsResult {
        entries,
        skipped_ids,
    })
}

fn process_data(values: &Vec<redis::Value>) -> Result<Vec<LogMessagePack>, RedisError> {
    let un_valued: Vec<Vec<u8>> = values
        .iter()
        .map(|e| match e {
            redis::Value::BulkString(x) => Ok(x.to_vec()),
            _ => Err(RedisError::Retryable(BAD_DATA.into())),
        })
        .collect::<Result<Vec<Vec<u8>>, RedisError>>()?;

    un_valued
        .iter()
        .map(|e| {
            let mut de = Deserializer::from_read_ref(&e);
            serde_path_to_error::deserialize::<_, LogMessagePack>(&mut de)
        })
        .collect::<Result<Vec<LogMessagePack>, serde_path_to_error::Error<_>>>()
        .map_err(|err| {
            println!("WARNING: Parse error in message {:?}", err);
            RedisError::Retryable(BAD_DATA.into())
        })
}

fn extract_records(messages: &Vec<LogMessagePack>) -> Vec<LogMsg> {
    messages
        .iter()
        .map(|e| e.bec_codec.data.log_msg.clone())
        .collect()
}

fn ack_logs(
    redis_conn: &mut RedisConnWithKey,
    config: &'static IngestorConfig,
    entry_ids: &[String],
) -> Result<(), RedisError> {
    if entry_ids.is_empty() {
        return Ok(());
    }
    let _: usize = redis_conn
        .conn
        .xack(&[redis_conn.key], &config.redis.consumer_group, entry_ids)
        .map_err(retryable_code)?;
    Ok(())
}

fn setup_consumer_group(
    mut conn: redis::Connection,
    config: &'static IngestorConfig,
    log_key: &'static str,
) -> Result<RedisConnWithKey, RedisError> {
    println!("INFO: Setting up consumer group");
    match conn.xgroup_create::<_, _, _, ()>(log_key, &config.redis.consumer_group, "0") {
        Ok(_) => {
            println!("INFO: Done setting up consumer group");
            Ok(RedisConnWithKey { conn, key: log_key })
        }
        Err(error) => {
            if let Some(code) = error.code()
                && code == "BUSYGROUP"
            {
                println!(
                    "INFO: Group {} already exists, rejoining with ID {}",
                    &config.redis.consumer_group, &config.redis.consumer_id
                );
                Ok(RedisConnWithKey { conn, key: log_key })
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
    mut redis_conn: redis::Connection,
    config: &'static IngestorConfig,
) -> Result<RedisConnWithKey, RedisError> {
    let user_key_exists = redis_conn
        .exists::<&str, bool>(USER_LOGGING_ENDPOINT[0])
        .map_err(fatal_code)?;
    let info_key_exists = redis_conn
        .exists::<&str, bool>(INFO_LOGGING_ENDPOINT[0])
        .map_err(fatal_code)?;
    if !user_key_exists && !info_key_exists {
        Err(RedisError::Retryable("No logging endpoint found".into()))
    } else {
        let key = if !user_key_exists {
            INFO_LOGGING_ENDPOINT[0]
        } else {
            USER_LOGGING_ENDPOINT[0]
        };
        setup_consumer_group(redis_conn, config, key)
    }
}

pub async fn producer_loop(
    tx: mpsc::UnboundedSender<RedisLogBatch>,
    mut ack_rx: mpsc::UnboundedReceiver<AckAction>,
    config: &'static IngestorConfig,
    max_retries: u8,
    initial_sleep: u64,
) -> Result<(), RedisError> {
    let mut redis_conn = create_redis_conn_with_retry(config, max_retries, initial_sleep)?;
    let mut stream_read_id: String = "0".into();
    println!("DEBUG: Starting Loki task producer loop");
    'main: loop {
        // Sleep between blocking calls prevents starvation of other tasks in thread limited environments
        sleep(Duration::from_millis(10)).await;
        match read_logs(&mut redis_conn, &stream_read_id, config) {
            Ok(read_result) => {
                if let Some(last_seen_id) = read_result
                    .skipped_ids
                    .last()
                    .cloned()
                    .or_else(|| read_result.entries.last().map(|(id, _)| id.clone()))
                {
                    stream_read_id = last_seen_id;
                }

                loop {
                    match ack_logs(&mut redis_conn, config, &read_result.skipped_ids) {
                        Ok(()) => break,
                        Err(e) => {
                            println!("ERROR: {:?}", e);
                            redis_conn =
                                create_redis_conn_with_retry(config, max_retries, initial_sleep)?;
                        }
                    }
                }

                if read_result.entries.is_empty() {
                    if stream_read_id != ">" {
                        stream_read_id = ">".into();
                    }
                    continue;
                }

                let packed: Vec<redis::Value> = read_result
                    .entries
                    .iter()
                    .map(|(_, value)| value.clone())
                    .collect();
                let entry_ids: Vec<String> = read_result
                    .entries
                    .iter()
                    .map(|(id, _)| id.clone())
                    .collect();
                let records = extract_records(&process_data(&packed).unwrap_or_else(|_| {
                    println!("WARNING: failed to process record: {:?}", &packed);
                    vec![error_log_item(packed.clone())]
                }));
                let batch = RedisLogBatch { entry_ids, records };

                if tx.send(batch).is_err() {
                    println!("INFO: Receiver dropped, stopping...");
                    break 'main Ok(());
                }

                let ack_action = match ack_rx.recv().await {
                    Some(ack_action) => ack_action,
                    None => {
                        println!("INFO: Ack receiver dropped, stopping...");
                        break 'main Ok(());
                    }
                };
                loop {
                    match ack_logs(&mut redis_conn, config, ack_action.entry_ids()) {
                        Ok(()) => break,
                        Err(e) => {
                            println!("ERROR: {:?}", e);
                            redis_conn =
                                create_redis_conn_with_retry(config, max_retries, initial_sleep)?;
                        }
                    }
                }
                if ack_action.should_stop() {
                    println!(
                        "ERROR: Acked poison log bundle after non-retryable Loki failure, stopping."
                    );
                    break 'main Err(RedisError::Fatal((
                        "Non-retryable Loki response for buffered logs".into(),
                        Box::new(None),
                    )));
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
    use crate::models::log_message::LogRecord;

    use super::*;

    #[test]
    fn test_error_log_item_contents() {
        let err_item = error_log_item(vec![]);
        assert_eq!(err_item.bec_codec.data.log_msg.record.level.name, "ERROR");
        assert_eq!(
            err_item.bec_codec.data.log_msg.record.message,
            "Error in ingestor processing log messages from Redis! Check log ingestor output for details."
        );
    }

    #[test]
    fn test_extract_records() {
        let mut pack = error_log_item(vec![]);
        pack.bec_codec.data.log_msg.record.message = "test".to_string();
        let records = extract_records(&vec![pack.clone()]);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].record.message, "test");
    }

    #[test]
    fn test_process_data_valid() {
        let pack = error_log_item(vec![]);
        let bytes = rmp_serde::to_vec(&pack).unwrap();
        let redis_val = redis::Value::BulkString(bytes.into());
        let result = process_data(&vec![redis_val]);
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
        let result = process_data(&vec![redis_val]);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_records_empty() {
        let records = extract_records(&vec![]);
        assert!(records.is_empty());
    }

    #[test]
    fn test_logrecord_serde_roundtrip() {
        let record = error_log_item(vec![]).bec_codec.data.log_msg.record.clone();
        let ser = serde_json::to_string(&record).unwrap();
        let de: LogRecord = serde_json::from_str(&ser).unwrap();
        assert_eq!(record, de);
    }
}
