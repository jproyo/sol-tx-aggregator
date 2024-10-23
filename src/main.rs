use anyhow::Result;
use sol_tx_aggregator::application::app;
use sol_tx_aggregator::application::Aggregator;
use sol_tx_aggregator::service;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let endpoint = "https://devnet.helius-rpc.com/?api-key=883a58ea-8640-456c-ad09-802120787faf";

    // Start the aggregator
    let aggregator = app(endpoint)?;
    tokio::spawn(async move {
        if let Err(e) = aggregator.run().await {
            tracing::error!("Aggregator error: {:?}", e);
            std::process::exit(1);
        }
    });

    // Start the API server
    service::api::start_server().await?;

    Ok(())
}
