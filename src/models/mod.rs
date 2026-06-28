pub mod log_message;

pub use log_message::{LogMessagePack, LogMsg, error_log_item};

#[derive(Debug, Clone)]
pub struct RedisLogBatch {
    pub entry_ids: Vec<String>,
    pub records: Vec<LogMsg>,
}

pub mod status_message;

pub use status_message::{ServiceStatus, StatusMessagePack};

pub mod dynamic_metric_message;
