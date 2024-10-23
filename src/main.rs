use anyhow::Result;
use sol_tx_aggregator::application::app;
use sol_tx_aggregator::application::Aggregator;
use sol_tx_aggregator::service;
use tokio::signal;
use tokio::sync::broadcast;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let endpoint = "https://devnet.helius-rpc.com/?api-key=883a58ea-8640-456c-ad09-802120787faf";

    // Create a shutdown channel
    let (shutdown_sender, _) = broadcast::channel(1);

    // Start the aggregator
    let aggregator = app(endpoint, shutdown_sender.clone())?;
    let aggregator_handle = tokio::spawn(async move {
        if let Err(e) = aggregator.run().await {
            tracing::error!("Aggregator error: {:?}", e);
        }
    });

    // Start the API server
    let server_handle = tokio::spawn(service::api::start_server(shutdown_sender.clone()));

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
