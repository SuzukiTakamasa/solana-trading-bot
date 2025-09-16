use chrono::{FixedOffset, TimeZone};
use chrono_tz::Asia::Tokyo;
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;
use crate::firestore::FirestoreDb;
use crate::trading::TradingState;

use anyhow::{Result, Context};
use serde::Serialize;
use std::sync::Arc;
use tracing::{info, error};

#[derive(Debug, Serialize)]
struct Message {
    #[serde(rename = "type")]
    message_type: String,
    text: String,
}

#[derive(Debug, Serialize)]
struct PushMessage {
    to: String,
    messages: Vec<Message>,
}

pub struct LineClient {
    client: reqwest::Client,
    channel_token: String,
    user_id: String
}

impl LineClient {
    pub fn new(channel_token: &str, user_id: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            channel_token: channel_token.to_string(),
            user_id: user_id.to_string(),
        }
    }
    
    pub async fn send_message(&self, text: &str) -> Result<()> {
        let message = Message {
            message_type: "text".to_string(),
            text: text.to_string(),
        };
        
        let push_message = PushMessage {
            to: self.user_id.to_string(),
            messages: vec![message],
        };
        
        let response = self.client
            .post("https://api.line.me/v2/bot/message/push")
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.channel_token))
            .json(&push_message)
            .send()
            .await
            .context("Failed to send LINE message")?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            error!("LINE API error: {}", error_text);
            anyhow::bail!("Failed to send LINE message: {}", error_text);
        }
        
        info!("LINE message sent successfully");
        Ok(())
    }

    pub async fn send_daily_high_and_low_sol_prices(
        &self,
        state: &TradingState,
        db: &Arc<FirestoreDb>) -> anyhow::Result<()> {
        let price_history = db.get_price_history(24).await?;
        
        if price_history.is_empty() {
            info!("No price history available for the last 24 hours.");
            return Ok(());
        }
        
        let mut high_price = Decimal::MIN;
        let mut low_price = Decimal::MAX;
        
        if let Some(max) = price_history.iter().map(|p| p.sol_price_usdc).max() {
            high_price = max;
        }
        if let Some(min) = price_history.iter().map(|p| p.sol_price_usdc).min() {
            low_price = min;
        }
        
        let last_trade_price = state.last_trade_price.unwrap_or(dec!(0));

        let message = format!(
            "📈 Daily SOL Price Update\n\n\
            High: {:.4}\n\
            Low: {:.4}\n\
            Last Trade Price: {:.4}\n\
            Time: {}",
            high_price * dec!(1_000_000_000),
            low_price * dec!(1_000_000_000),
            last_trade_price * dec!(1_000_000_000),
            Tokyo.from_utc_datetime(&chrono::Utc::now().naive_utc()).with_timezone(&FixedOffset::east_opt(9 * 3600).unwrap()).format("%Y-%m-%d %H:%M:%S JST")
        );
        
        self.send_message(&message).await
    }
    
    pub async fn send_success_notification(
        &self,
        state: &TradingState,
        profit: Decimal,
    ) -> anyhow::Result<()> {

        let trade_price = state.last_trade_price.unwrap_or(dec!(0));
        let message = format!(
            "😎 Trade executed!\n\
            Position: {}\n\
            Trade Price: {:.4} USDC\n\
            Profit: {:.4} USDC\n\
            Time: {}",
            state.position,
            trade_price * dec!(1_000_000_000),
            profit,
            Tokyo.from_utc_datetime(&chrono::Utc::now().naive_utc()).with_timezone(&FixedOffset::east_opt(9 * 3600).unwrap()).format("%Y-%m-%d %H:%M:%S JST")
        );
        info!("{}", message);
        self.send_message(&message).await
    }
    
    pub async fn send_error_notification(
        &self,
        e: &anyhow::Error,
    ) -> Result<()> {
        let message = format!(
            "🥺 Trading error...\n\
            {}\n\
            Time: {}",
            e,
            Tokyo.from_utc_datetime(&chrono::Utc::now().naive_utc()).with_timezone(&FixedOffset::east_opt(9 * 3600).unwrap()).format("%Y-%m-%d %H:%M:%S JST")
        );
        error!("Trading error: {}", e);
        self.send_message(&message).await
    }
    
    
}