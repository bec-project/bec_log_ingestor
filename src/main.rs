use std::process::exit;

use tokio::sync::mpsc;

mod models;
use crate::models::LogMsg;

mod status_message;

mod config;
use crate::config::{IngestorConfig, assemble_config};

mod redis_logs;
use crate::redis_logs::producer_loop;

mod loki_push;
use crate::loki_push::consumer_loop;

mod metrics;
mod metrics_core;
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

async fn run_services(config: &'static IngestorConfig) {
    println!("Starting log ingestor with config: \n {:?}", &config);

    let metrics = if config.enable_metrics {
        println!("Starting metrics task...");
        tokio::spawn(metrics_loop(config))
    } else {
        tokio::spawn(futures::future::ready(()))
    };

    if config.enable_logging {
        println!("Starting log ingestor task...");
        let (tx, mut rx) = mpsc::unbounded_channel::<LogMsg>();
        let producer = tokio::spawn(producer_loop(tx, config));
        consumer_loop(&mut rx, config).await;
        let _ = tokio::join!(producer);
    } else {
        _ = tokio::join!(metrics);
    }
}

#[tokio::main]
async fn main() {
    let config = Box::leak(Box::new(assemble_config(config_paths())));
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");
    run_services(config).await
}
