mod aggregator;
mod api;
mod models;

use anyhow::Result;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Start the aggregator
    let aggregator = aggregator::Aggregator::new().await?;
    tokio::spawn(async move {
        if let Err(e) = aggregator.run().await {
            tracing::error!("Aggregator error: {:?}", e);
        }
    });

    // Start the API server
    api::start_server().await?;

    Ok(())
}
