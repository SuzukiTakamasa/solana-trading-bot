use anyhow::{Result, Context};
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    // Solana configuration
    pub rpc_url: String,
    pub private_key: String,
    
    // Jupiter configuration
    pub jupiter_api_url: String,
    pub slippage_bps: u16,
    
    // LINE bot configuration
    pub line_channel_token: String,
    pub line_user_id: String,
    
    // Token addresses
    pub sol_mint: String,
    pub usdc_mint: String,
    
    // Server configuration
    pub port: u16,
    
    // Firestore configuration
    pub gcp_project_id: String,
    pub data_retention_days: u32,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        Ok(Config {
            rpc_url: env::var("SOLANA_RPC_URL")
                .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string()),
            
            private_key: env::var("WALLET_PRIVATE_KEY")
                .context("WALLET_PRIVATE_KEY must be set")?,
            
            jupiter_api_url: env::var("JUPITER_API_URL")
                .unwrap_or_else(|_| "https://lite-api.jup.ag/swap/v1".to_string()),
            
            slippage_bps: env::var("SLIPPAGE_BPS")
                .unwrap_or_else(|_| "50".to_string())
                .parse()
                .context("Invalid SLIPPAGE_BPS")?,
            
            line_channel_token: env::var("LINE_CHANNEL_TOKEN")
                .context("LINE_CHANNEL_TOKEN must be set")?,
            
            line_user_id: env::var("LINE_USER_ID")
                .context("LINE_USER_ID must be set")?,
            
            sol_mint: env::var("SOL_MINT")
                .unwrap_or_else(|_| "So11111111111111111111111111111111111111112".to_string()),
            
            usdc_mint: env::var("USDC_MINT")
                .unwrap_or_else(|_| "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string()),
            
            port: env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .context("Invalid PORT")?,
            
            gcp_project_id: env::var("GCP_PROJECT_ID")
                .context("GCP_PROJECT_ID must be set")?,
            
            data_retention_days: env::var("DATA_RETENTION_DAYS")
                .unwrap_or_else(|_| "365".to_string())
                .parse()
                .context("Invalid DATA_RETENTION_DAYS")?,
        })
    }
}
