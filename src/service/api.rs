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
use solana_sdk::pubkey::Pubkey;
use std::{str::FromStr, sync::Arc};
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
    sender: Option<String>,
    receiver: Option<String>,
    slot: Option<u64>,
}

#[derive(Deserialize)]
struct AccountQuery {
    address: Option<String>,
}

async fn get_transactions(
    State(app_state): State<Arc<impl Application>>,
    Query(params): Query<TransactionQuery>,
) -> Result<Json<Vec<Transaction>>, StatusCode> {
    if let Some(id) = params.id {
        return get_transaction_by_id(&app_state, id).await;
    }

    if let Some(day) = params.day {
        return get_transactions_by_date(day, &app_state).await;
    }

    if let Some(sender) = params.sender {
        return get_transactions_by_sender(sender, &app_state).await;
    }

    if let Some(receiver) = params.receiver {
        return get_transactions_by_receiver(receiver, &app_state).await;
    }

    if let Some(slot) = params.slot {
        return get_transactions_by_slot(slot, &app_state).await;
    }

    get_all_transactions(&app_state).await
}

async fn get_transactions_by_sender(
    sender: String,
    app_state: &Arc<impl Application>,
) -> Result<Json<Vec<Transaction>>, StatusCode> {
    let pubkey = Pubkey::from_str(&sender).map_err(|_| StatusCode::BAD_REQUEST)?;
    app_state
        .get_transactions_by_sender(pubkey)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn get_transactions_by_receiver(
    receiver: String,
    app_state: &Arc<impl Application>,
) -> Result<Json<Vec<Transaction>>, StatusCode> {
    let pubkey = Pubkey::from_str(&receiver).map_err(|_| StatusCode::BAD_REQUEST)?;
    app_state
        .get_transactions_by_receiver(pubkey)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn get_transactions_by_slot(
    slot: u64,
    app_state: &Arc<impl Application>,
) -> Result<Json<Vec<Transaction>>, StatusCode> {
    app_state
        .get_transactions_by_slot(slot)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn get_all_transactions(
    app_state: &Arc<impl Application>,
) -> Result<Json<Vec<Transaction>>, StatusCode> {
    app_state
        .get_transactions()
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn get_transactions_by_date(
    day: String,
    app_state: &Arc<impl Application>,
) -> Result<Json<Vec<Transaction>>, StatusCode> {
    let date = DateTime::parse_from_str(&day, "%Y-%m-%d").map_err(|_| StatusCode::BAD_REQUEST)?;
    app_state
        .get_transactions_by_date(date.format("%Y-%m-%d").to_string())
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn get_transaction_by_id(
    app_state: &Arc<impl Application>,
    id: String,
) -> Result<Json<Vec<Transaction>>, StatusCode> {
    app_state
        .as_ref()
        .get_transaction(id)
        .await
        .map(|v| Json(vec![v]))
        .map_err(|_| StatusCode::NOT_FOUND)
}

async fn get_accounts(
    State(app_state): State<Arc<impl Application>>,
    Query(params): Query<AccountQuery>,
) -> Result<Json<Vec<Account>>, StatusCode> {
    if let Some(address) = params.address {
        let pubkey = Pubkey::from_str(&address).map_err(|_| StatusCode::BAD_REQUEST)?;
        app_state
            .get_account(pubkey)
            .await
            .map(|v| Json(vec![v]))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    } else {
        app_state
            .get_accounts()
            .await
            .map(Json)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    }
}
