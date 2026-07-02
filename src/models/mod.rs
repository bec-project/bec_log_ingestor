pub mod log_message;

pub use log_message::{LogMessagePack, LogMsg, error_log_item};

#[derive(Debug, Clone, PartialEq)]
pub enum AckAction {
    Ack(Vec<String>),
    AckAndStop(Vec<String>),
}

impl AckAction {
    pub fn entry_ids(&self) -> &[String] {
        match self {
            AckAction::Ack(ids) | AckAction::AckAndStop(ids) => ids,
        }
    }

    pub fn should_stop(&self) -> bool {
        matches!(self, AckAction::AckAndStop(_))
    }
}

#[derive(Debug, Clone)]
pub struct RedisLogBatch {
    pub entry_ids: Vec<String>,
    pub records: Vec<LogMsg>,
}

pub mod status_message;

pub use status_message::{ServiceStatus, StatusMessagePack};

pub mod dynamic_metric_message;
