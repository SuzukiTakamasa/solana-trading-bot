mod config;
mod firestore;
mod jupiter;
mod line_bot;
mod service;
mod trading;
mod wallet;

use anyhow::Result;
use axum::{extract::Query, response::IntoResponse, routing::get, Json, Router};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{info, error};
use chrono::{FixedOffset, TimeZone};
use chrono::Timelike;
use chrono_tz::Asia::Tokyo;

use trading::Position;


#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "solana_trading_bot=info".into()),
        )
        .init();

    // Load configuration
    let config = config::Config::from_env()?;
    info!("Configuration loaded successfully");

    // Start HTTP server
    let app = Router::new()
        .route("/", get(health_check))
        .route("/health", get(health_check))
        .route("/trigger", get(trigger_trade))
        .route("/api/performance", get(get_performance))
        .route("/api/price-history", get(get_price_history))
        .route("/api/trading-sessions", get(get_trading_sessions));

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> impl IntoResponse {
    "OK"
}

async fn trigger_trade() -> impl IntoResponse {
    info!("Trade trigger received");
    
    // Spawn a task to handle the trade
    tokio::spawn(async {
        if let Err(e) = execute_single_trade().await {
            error!("Trade execution error: {}", e);
        }
    });
    
    "Trade triggered"
}

async fn execute_single_trade() -> Result<()> {
    let config = config::Config::from_env()?;
    let wallet = wallet::Wallet::new(&config.private_key)?;
    let line_client = line_bot::LineClient::new(&config.line_channel_token, &config.line_user_id);
    
    // Initialize Firestore if configured
    let firestore = match firestore::FirestoreDb::new(config.gcp_project_id.clone()).await {
        Ok(db) => Some(Arc::new(db)),
        Err(e) => {
            error!("Failed to initialize Firestore: {}", e);
            None
        }
    };
    
    // Initialize trading state with persistent storage
    let mut state = trading::TradingState::new();
    if let Some(db) = firestore.clone() {
        state = state.with_firestore(db.clone());
        if let Err(e) = state.load_from_firestore().await {
            error!("Failed to load trading state from Firestore: {}", e);
        }
        
        // Cleanup old data (this replaces the periodic cleanup task)
        if let Err(e) = db.cleanup_old_data(config.data_retention_days).await {
            error!("Failed to cleanup old data: {}", e);
        }
    }
    
    /*
     Check if initial swap is needed (first trade)
     if state.total_trades == 0 {
        info!("Performing initial swap: SOL -> USDC");
        match trading::perform_initial_swap(&wallet, &config).await {
            Ok(_) => {
                line_client.send_startup_notification(&config.line_user_id).await?;
            }
            Err(e) => {
                line_client.send_error_notification(&config.line_user_id, &format!("{}", e)).await?;
                return Err(e);
            }
        }
    }
    */
    
    // Execute the trade
    match trading::check_and_trade(&wallet, &config, &mut state).await {
        Ok(Some(profit)) => {
            let profit_unit = match state.position {
                Position::SOL => "USDC",
                Position::USDC => "SOL",
            };
            let message = format!(
                "😎 Trade executed!\n\
                Position: {0}\n\
                Profit: {1:.4} {2}\n\
                Total: {3:.4} USDC\n\
                Time: {4}",
                state.position,
                profit,
                profit_unit,
                state.total_profit_usdc,
                Tokyo.from_utc_datetime(&chrono::Utc::now().naive_utc()).with_timezone(&FixedOffset::east_opt(9 * 3600).unwrap()).format("%Y-%m-%d %H:%M:%S JST")
            );
            info!("{}", message);
            line_client.send_message(&message).await?;
        }
        Ok(None) => {
            info!("No trading opportunity found");
        }
        Err(e) => {
            let message = format!(
                "🥺 Trading error...\n\
                {}\n\
                Time: {}",
                e,
                Tokyo.from_utc_datetime(&chrono::Utc::now().naive_utc()).with_timezone(&FixedOffset::east_opt(9 * 3600).unwrap()).format("%Y-%m-%d %H:%M:%S JST")
            );
            error!("Trading error: {}", e);
            line_client.send_message(&message).await?;
            return Err(e);
        }
    }

    // Send daily high/low price update at midnight JST
    let now_jst = chrono::Utc::now().with_timezone(&Tokyo);
    if now_jst.hour() == 0 && now_jst.minute() == 0 {
        // Send daily price update at midnight JST
        if let Some(db) = firestore {
            if let Err(e) = line_client.send_daily_high_and_low_sol_prices(&db).await {
                error!("Failed to send daily price update: {}", e);
            }
        }
    }
    
    Ok(())
}


#[derive(Deserialize)]
struct PerformanceQuery {
    days: Option<u32>,
}

#[derive(Serialize)]
struct PerformanceResponse {
    total_trades: i64,
    winning_trades: i64,
    losing_trades: i64,
    total_profit_loss: String,
    total_gas_fees: String,
    win_rate: String,
    period_days: u32,
}

async fn get_performance(Query(params): Query<PerformanceQuery>) -> impl IntoResponse {
    let days = params.days.unwrap_or(30);
    
    match get_trading_performance_internal(days).await {
        Ok(performance) => Json(PerformanceResponse {
            total_trades: performance.total_trades,
            winning_trades: performance.winning_trades,
            losing_trades: performance.losing_trades,
            total_profit_loss: performance.total_profit_loss.to_string(),
            total_gas_fees: performance.total_gas_fees.to_string(),
            win_rate: format!("{:.2}%", performance.win_rate),
            period_days: performance.period_days,
        }).into_response(),
        Err(e) => {
            error!("Failed to get performance data: {}", e);
            format!("Error: {}", e).into_response()
        }
    }
}

#[derive(Deserialize)]
struct PriceHistoryQuery {
    hours: Option<u32>,
}

async fn get_price_history(Query(params): Query<PriceHistoryQuery>) -> impl IntoResponse {
    let hours = params.hours.unwrap_or(24);
    
    match get_price_history_internal(hours).await {
        Ok(prices) => Json(prices).into_response(),
        Err(e) => {
            error!("Failed to get price history: {}", e);
            format!("Error: {}", e).into_response()
        }
    }
}

#[derive(Deserialize)]
struct TradingSessionsQuery {
    limit: Option<u32>,
}

async fn get_trading_sessions(Query(params): Query<TradingSessionsQuery>) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(50);
    
    match get_trading_sessions_internal(limit).await {
        Ok(sessions) => Json(sessions).into_response(),
        Err(e) => {
            error!("Failed to get trading sessions: {}", e);
            format!("Error: {}", e).into_response()
        }
    }
}

async fn get_trading_performance_internal(days: u32) -> Result<firestore::TradingPerformance> {
    let config = config::Config::from_env()?;
    let db = firestore::FirestoreDb::new(config.gcp_project_id).await?;
    db.get_trading_performance(days).await
}

async fn get_price_history_internal(hours: u32) -> Result<Vec<firestore::PriceHistory>> {
    let config = config::Config::from_env()?;
    let db = firestore::FirestoreDb::new(config.gcp_project_id).await?;
    db.get_price_history(hours).await
}

async fn get_trading_sessions_internal(limit: u32) -> Result<Vec<firestore::TradingSession>> {
    let config = config::Config::from_env()?;
    let db = firestore::FirestoreDb::new(config.gcp_project_id).await?;
    
    // Get all trading sessions from the database
    let _all_sessions = db.get_trading_performance(3650).await?; // Get up to 10 years of data
    
    // Get the actual sessions data by making another call
    // For now, we'll use the price history method as a template
    let url = format!(
        "https://firestore.googleapis.com/v1/projects/{}/databases/{}/documents/trading_sessions?orderBy=timestamp%20desc",
        db.project_id, db.database_id
    );
    
    let auth_token = db.get_auth_token().await?;
    
    let response = reqwest::Client::new()
        .get(&url)
        .header("Authorization", auth_token)
        .send()
        .await?
        .error_for_status()?;
    
    #[derive(serde::Deserialize)]
    struct ListResponse {
        documents: Option<Vec<firestore::FirestoreDocument>>,
    }
    
    let result: ListResponse = response.json().await?;
    let mut sessions = Vec::new();
    
    if let Some(documents) = result.documents {
        for doc in documents.into_iter().take(limit as usize) {
            match db.firestore_document_to_json(doc) {
                Ok(session) => sessions.push(session),
                Err(e) => {
                    error!("Error reading trading session document: {}", e);
                    continue;
                }
            }
        }
    }
    
    Ok(sessions)
}