use std::process::exit;

use tokio::sync::mpsc;

mod models;
use crate::models::LogMsg;

mod redis_logs;
use crate::redis_logs::producer_loop;

mod elastic_push;
use crate::elastic_push::consumer_loop;

mod config;
use crate::config::IngestorConfig;

use clap::Parser;

#[derive(clap::Parser, Debug)]
struct Args {
    /// Specify a config file
    #[arg(short = 'c', long = "config")]
    config: std::path::PathBuf,
}

fn config_path() -> std::path::PathBuf {
    let args = Args::parse();
    if !args.config.exists() {
        println!(
            "Error, config file not found at {:?}",
            args.config.as_path()
        );
        exit(1)
    }
    args.config
}

fn entry() -> IngestorConfig {
    let path = config_path();
    IngestorConfig::from_file(path)
}

async fn main_loop(config: IngestorConfig) {
    println!("Starting log ingestor with config: \n {:?}", &config);

    let (tx, mut rx) = mpsc::unbounded_channel::<LogMsg>();
    let producer = tokio::spawn(producer_loop(tx, config.redis.clone()));
    consumer_loop(&mut rx, config.elastic.clone()).await;

    let _ = tokio::join!(producer);
}

#[tokio::main]
async fn main() {
    let config = entry();
    main_loop(config).await
}
