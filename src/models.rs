use serde_derive::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Deserialize, Serialize, Clone)]
pub struct Elapsed {
    pub repr: String,
    pub seconds: f64,
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Clone)]
pub struct File {
    pub name: String,
    pub path: String,
}
#[derive(Debug, PartialEq, Deserialize, Serialize, Clone)]
pub struct LogLevel {
    pub icon: String,
    pub name: String,
    pub no: usize,
}
#[derive(Debug, PartialEq, Deserialize, Serialize, Clone)]
pub struct NameId {
    pub name: String,
    pub id: usize,
}
#[derive(Debug, PartialEq, Deserialize, Serialize, Clone)]
pub struct Timestamp {
    pub repr: String,
    pub timestamp: f64,
}

impl Timestamp {
    pub fn as_epoch_nanos(&self) -> String {
        ((self.timestamp * 1_000_000.0) as i64).to_string()
    }
}
#[derive(Debug, PartialEq, Deserialize, Serialize, Clone)]
pub struct LogRecord {
    pub elapsed: Elapsed,
    pub exception: Option<serde_json::Value>,
    pub extra: serde_json::Value,
    pub file: File,
    pub function: String,
    pub level: LogLevel,
    pub line: usize,
    pub message: String,
    pub module: String,
    pub name: String,
    pub process: NameId,
    pub thread: NameId,
    pub time: Timestamp,
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Clone)]
pub struct LogMsg {
    pub record: LogRecord,
    pub service_name: String,
    pub text: String,
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Clone)]
pub struct LogMessage {
    pub log_type: String,
    pub log_msg: LogMsg,
    pub metadata: serde_json::Value,
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Clone)]
pub struct LogMessagePackInternal {
    pub encoder_name: String,
    pub type_name: String,
    pub data: LogMessage,
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Clone)]
pub struct LogMessagePack {
    #[serde(rename = "__bec_codec__")]
    pub bec_codec: LogMessagePackInternal,
}

pub fn error_log_item() -> LogMessagePack {
    LogMessagePack {
        bec_codec: LogMessagePackInternal {
            encoder_name: "".into(),
            type_name: "".into(),
            data: LogMessage {
                log_type: "".into(),
                log_msg: LogMsg {
                    record: LogRecord {
                        elapsed: Elapsed {
                            repr: "".into(),
                            seconds: 0.0,
                        },
                        exception: None,
                        extra: {}.into(),
                        file: File {
                            name: "".into(),
                            path: "".into(),
                        },
                        function: "".into(),
                        level: LogLevel {
                            icon: "".into(),
                            name: "ERROR".into(),
                            no: 100,
                        },
                        line: 0,
                        message: "Error processing log messages from Redis!".into(),
                        module: "".into(),
                        name: "".into(),
                        process: NameId {
                            name: "".into(),
                            id: 0,
                        },
                        thread: NameId {
                            name: "".into(),
                            id: 0,
                        },
                        time: Timestamp {
                            repr: "".into(),
                            timestamp: 0.0,
                        },
                    },
                    service_name: "".into(),
                    text: "".into(),
                },
                metadata: {}.into(),
            },
        },
    }
}
