use crate::{
    application::app::Application,
    domain::models::{Account, Transaction},
};
use anyhow::Ok;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use chrono::DateTime;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;

pub async fn start_server(
    shutdown: broadcast::Sender<()>,
    app: Arc<impl Application + Send + Sync + 'static>,
) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/transactions", get(get_transactions))
        .route("/accounts", get(get_accounts))
        .with_state(app)
        .layer(CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    let server = axum::serve(listener, app);

    tracing::info!("API server started on port 3000");

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

async fn get_transactions(
    State(app_state): State<Arc<impl Application>>,
    Query(params): Query<TransactionQuery>,
) -> Result<Json<Vec<Transaction>>, StatusCode> {
    if let Some(id) = params.id {
        let transactions = app_state
            .as_ref()
            .get_transaction(id)
            .await
            .map(|v| Json(vec![v]))
            .map_err(|_| StatusCode::NOT_FOUND);
        return transactions;
    }

    if let Some(day) = params.day {
        let date =
            DateTime::parse_from_str(&day, "%Y-%m-%d").map_err(|_| StatusCode::BAD_REQUEST)?;
        let transactions = app_state
            .get_transactions_by_date(date.format("%Y-%m-%d").to_string())
            .await
            .map(|v| Json(v))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
        return transactions;
    } else {
        let transactions = app_state
            .get_transactions()
            .await
            .map(|v| Json(v))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
        return transactions;
    }
}

async fn get_accounts(
    State(app_state): State<Arc<impl Application>>,
) -> Result<Json<Vec<Account>>, StatusCode> {
    let accounts = app_state
        .get_accounts()
        .await
        .map(|v| Json(v))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
    accounts
}
