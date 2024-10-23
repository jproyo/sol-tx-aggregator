use axum::{extract::Query, routing::get, Json, Router};
use serde::Deserialize;
use tower_http::cors::CorsLayer;

use crate::domain::models::{Account, Transaction};

pub async fn start_server() -> anyhow::Result<()> {
    let app = Router::new()
        .route("/transactions", get(get_transactions))
        .route("/accounts", get(get_accounts))
        .layer(CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    axum::serve(listener, app).await?;

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
