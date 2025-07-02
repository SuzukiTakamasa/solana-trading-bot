use anyhow::{Result, Context};
use serde::Serialize;
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
    /*
    pub async fn send_error_notification(
        &self,
        error: &str,
    ) -> Result<()> {
        let message = format!(
            "❌ Trading Bot Error\n\n\
            Error: {}\n\
            Time: {}",
            error,
            Tokyo.from_utc_datetime(&chrono::Utc::now().naive_utc()).with_timezone(&FixedOffset::east_opt(9 * 3600).unwrap()).format("%Y-%m-%d %H:%M:%S JST")
        );
        
        self.send_message(&message).await
    }
    
    pub async fn send_startup_notification(
        &self,
    ) -> Result<()> {
        let message = format!(
            "🚀 Trading Bot Started!\n\n\
            Strategy: SOL-USDC Hourly Trading\n\
            Time: {}",
            Tokyo.from_utc_datetime(&chrono::Utc::now().naive_utc()).with_timezone(&FixedOffset::east_opt(9 * 3600).unwrap()).format("%Y-%m-%d %H:%M:%S JST")
        );
        
        self.send_message(&message).await
    }
    */
    
}