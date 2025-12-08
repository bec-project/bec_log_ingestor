use std::process::exit;

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
use crate::metrics::metrics_loop;

use clap::Parser;

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
            "Error, config file not found at {:?}",
            args.config.as_path()
        );
        exit(1)
    }
    if let Some(ref metric_path) = args.metrics_config {
        if !metric_path.exists() {
            println!(
                "Error, metrics config file not found at {:?}",
                metric_path.as_path()
            );
            exit(1)
        }
    }
    (args.config, args.metrics_config)
}

async fn main_loop(config: IngestorConfig) {
    println!("Starting log ingestor with config: \n {:?}", &config);

    let metrics = tokio::spawn(metrics_loop(config.clone()));

    let (tx, mut rx) = mpsc::unbounded_channel::<LogMsg>();
    let producer = tokio::spawn(producer_loop(tx, config.redis.clone()));

    consumer_loop(&mut rx, config.loki.clone()).await;
    let _ = tokio::join!(producer);
}

#[tokio::main]
async fn main() {
    main_loop(assemble_config(config_paths())).await
}
