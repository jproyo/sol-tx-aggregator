use anyhow::Result;
use clap::{arg, command, Parser};
use sol_tx_aggregator::application::app;
use sol_tx_aggregator::application::app::Application;
use sol_tx_aggregator::service;
use std::sync::Arc;
use tokio::signal;
use tokio::sync::broadcast;

#[derive(Parser, Debug)]
#[command(
    version,
    about,
    long_about = "Solana transaction aggregator with REST API"
)]
struct AggProgram {
    /// RPC endpoint
    #[arg(short, long)]
    rpc_endpoint: String,

    /// Number of retries
    #[arg(short, long, default_value_t = 3)]
    num_retries: usize,

    /// Listen port REST API
    #[arg(short, long, default_value_t = 3000)]
    listen_port: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let args = AggProgram::parse();

    // Create a shutdown channel
    let (shutdown_sender, _) = broadcast::channel(1);

    // Start the aggregator
    let app = Arc::new(app::App::new());
    let shutdown_sender_aggregator = shutdown_sender.clone();
    let app_clone = app.clone();
    let aggregator_handle = tokio::spawn(async move {
        if let Err(e) = app_clone
            .run_aggregator(
                &args.rpc_endpoint,
                args.num_retries,
                shutdown_sender_aggregator,
            )
            .await
        {
            tracing::error!("Aggregator error: {:?}", e);
        }
    });

    // Start the API server
    let server_handle = tokio::spawn(service::api::start_server(
        shutdown_sender.clone(),
        app.clone(),
        args.listen_port,
    ));

    // Wait for shutdown signal
    tokio::select! {
        _ = signal::ctrl_c() => {
            shutdown_sender.send(()).unwrap();
            tracing::warn!("Received Ctrl+C, shutting down...");
        }
    }

    shutdown_sender.send(()).unwrap();

    // Wait for tasks to complete
    let _ = tokio::join!(aggregator_handle, server_handle);

    tracing::info!("Shutdown complete");
    Ok(())
}
