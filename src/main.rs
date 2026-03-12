use clap::Parser;
use std::process::exit;
use std::sync::atomic::AtomicBool;
use tokio::sync::mpsc;

mod models;
use crate::models::LogMsg;

mod config;
use crate::config::{IngestorConfig, assemble_config};

mod redis_logs;
use crate::redis_logs::producer_loop;

mod loki_push;
use crate::loki_push::consumer_loop;

mod metrics;
mod metrics_core;
use crate::metrics::{metric_definitions, metrics_loop};

#[cfg(test)]
mod tests;

static STOP_METRICS: AtomicBool = AtomicBool::new(false);
// Final maximum sleep time of ~80s after 256*0.3 s
const MAX_RETRIES: u8 = 8;
const INITIAL_SLEEP: u64 = 300;

#[derive(clap::Parser, Debug)]
struct Args {
    /// Specify a config file
    #[arg(short = 'c', long = "config")]
    config: std::path::PathBuf,
    #[arg(short = 'm', long = "metrics_config")]
    metrics_config: Option<std::path::PathBuf>,
}

fn config_paths() -> (std::path::PathBuf, Option<std::path::PathBuf>) {
    let args = Args::parse();
    if !args.config.exists() {
        println!(
            "ERROR: config file not found at {:?}",
            args.config.as_path()
        );
        exit(1)
    }
    if let Some(ref metric_path) = args.metrics_config
        && !metric_path.exists()
    {
        println!(
            "ERROR: metrics config file not found at {:?}",
            metric_path.as_path()
        );
        exit(1)
    }
    (args.config, args.metrics_config)
}

async fn run_services(config: &'static IngestorConfig) {
    println!("INFO: Starting log ingestor with config: \n {:?}", &config);

    let metrics = if config.enable_metrics {
        println!("DEBUG: Starting metrics task...");
        tokio::spawn(metrics_loop(config, metric_definitions(config)))
    } else {
        tokio::spawn(futures::future::ready(()))
    };

    if config.enable_logging {
        println!("DEBUG: Starting log ingestor task...");
        let (tx, mut rx) = mpsc::unbounded_channel::<LogMsg>();
        let producer = tokio::spawn(producer_loop(tx, config, MAX_RETRIES, INITIAL_SLEEP));
        consumer_loop(&mut rx, config).await;
        let _ = tokio::join!(producer);
    } else {
        _ = tokio::join!(metrics);
    }

    exit(69) // SERVICE UNAVAILABLE
}

#[tokio::main]
async fn main() {
    let config = Box::leak(Box::new(assemble_config(config_paths())));
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("ERROR: Failed to install rustls crypto provider");
    run_services(config).await
}
