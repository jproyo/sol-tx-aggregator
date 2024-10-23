use axum::{extract::Query, routing::get, Json, Router};
use serde::Deserialize;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;

use crate::domain::models::{Account, Transaction};

pub async fn start_server(shutdown: broadcast::Sender<()>) -> Result<(), std::io::Error> {
    let app = Router::new()
        .route("/transactions", get(get_transactions))
        .route("/accounts", get(get_accounts))
        .layer(CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    let server = axum::serve(listener, app);

    let mut shutdown_rx = shutdown.subscribe();

    tokio::select! {
        _ = shutdown_rx.recv() => {
            tracing::warn!("API server received shutdown signal");
        }
        _ = server => {
            tracing::warn!("API server stopped unexpectedly");
        }
    }

    Ok(())
}

#[derive(Deserialize)]
struct TransactionQuery {
    id: Option<String>,
    day: Option<String>,
}

async fn get_transactions(Query(params): Query<TransactionQuery>) -> Json<Vec<Transaction>> {
    // Implement logic to fetch transactions based on query parameters
    // This is a placeholder and needs to be implemented
    Json(vec![])
}

async fn get_accounts() -> Json<Vec<Account>> {
    // Implement logic to fetch accounts
    // This is a placeholder and needs to be implemented
    Json(vec![])
}
