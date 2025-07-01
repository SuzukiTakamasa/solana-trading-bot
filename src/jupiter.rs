use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use solana_sdk::{
    pubkey::Pubkey,
    transaction::{Transaction, VersionedTransaction},
};
use solana_client::rpc_client::RpcClient;
use tracing::{info, error};

// QuoteRequest is not needed as we use query parameters directly

#[derive(Debug, Serialize, Deserialize)]
pub struct QuoteResponse {
    #[serde(rename = "inputMint")]
    pub input_mint: String,
    #[serde(rename = "inAmount")]
    pub in_amount: u64,
    #[serde(rename = "outputMint")]
    pub output_mint: String,
    #[serde(rename = "outAmount")]
    pub out_amount: u64,
    #[serde(rename = "otherAmountThreshold")]
    pub other_amount_threshold: u64,
    #[serde(rename = "swapMode")]
    pub swap_mode: String,
    #[serde(rename = "slippageBps")]
    pub slippage_bps: u16,
    #[serde(rename = "platformFee")]
    pub platform_fee: Option<PlatformFee>,
    #[serde(rename = "priceImpactPct")]
    pub price_impact_pct: String,
    #[serde(rename = "routePlan")]
    pub route_plan: Vec<RoutePlanStep>,
    #[serde(rename = "contextSlot")]
    pub context_slot: Option<u64>,
    #[serde(rename = "timeTaken")]
    pub time_taken: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlatformFee {
    pub amount: String,
    #[serde(rename = "feeBps")]
    pub fee_bps: u16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RoutePlanStep {
    #[serde(rename = "swapInfo")]
    pub swap_info: SwapInfo,
    pub percent: u8,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SwapInfo {
    #[serde(rename = "ammKey")]
    pub amm_key: String,
    pub label: String,
    #[serde(rename = "inputMint")]
    pub input_mint: String,
    #[serde(rename = "outputMint")]
    pub output_mint: String,
    #[serde(rename = "inAmount")]
    pub in_amount: String,
    #[serde(rename = "outAmount")]
    pub out_amount: String,
    #[serde(rename = "feeAmount")]
    pub fee_amount: String,
    #[serde(rename = "feeMint")]
    pub fee_mint: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PriorityFeeLevel {
    #[serde(rename = "maxLamports")]
    pub max_lamports: u64,
    #[serde(rename = "priorityLevel")]
    pub priority_level: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PrioritizationFee {
    Auto(String),
    Manual {
        #[serde(rename = "priorityLevelWithMaxLamports")]
        priority_level_with_max_lamports: PriorityFeeLevel,
    },
}

#[derive(Debug, Serialize)]
pub struct SwapRequest {
    #[serde(rename = "quoteResponse")]
    pub quote_response: QuoteResponse,
    #[serde(rename = "userPublicKey")]
    pub user_public_key: String,
    #[serde(rename = "wrapAndUnwrapSol")]
    pub wrap_and_unwrap_sol: bool,
    #[serde(rename = "dynamicComputeUnitLimit")]
    pub dynamic_compute_unit_limit: bool,
    #[serde(rename = "dynamicSlippage")]
    pub dynamic_slippage: bool,
    #[serde(rename = "prioritizationFeeLamports")]
    pub prioritization_fee_lamports: Option<PrioritizationFee>,
    #[serde(rename = "useTokenLedger")]
    pub use_token_ledger: bool,
}

#[derive(Debug, Deserialize)]
pub struct SwapResponse {
    #[serde(rename = "swapTransaction")]
    pub swap_transaction: String,
    #[serde(rename = "lastValidBlockHeight")]
    pub last_valid_block_height: u64,
}

pub struct JupiterClient {
    client: reqwest::Client,
    api_url: String,
}

impl JupiterClient {
    pub fn new(api_url: &str) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("solana-trading-bot/1.0")
            .build()
            .expect("Failed to create HTTP client");
            
        Self {
            client,
            api_url: api_url.to_string(),
        }
    }
    
    pub async fn get_quote(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount: u64,
        slippage_bps: u16,
    ) -> Result<QuoteResponse> {
        let url = format!("{}/quote", self.api_url);
        
        info!("Requesting quote from Jupiter API: {}", url);
        info!("Parameters - inputMint: {}, outputMint: {}, amount: {}, slippageBps: {}", 
            input_mint, output_mint, amount, slippage_bps);
        
        let response = self.client
            .get(&url)
            .query(&[
                ("inputMint", input_mint),
                ("outputMint", output_mint),
                ("amount", &amount.to_string()),
                ("slippageBps", &slippage_bps.to_string()),
                ("swapMode", "ExactIn"),
                ("restrictIntermediateTokens", "false"),
                ("onlyDirectRoutes", "false"),
                ("maxAccounts", "64"),
            ])
            .send()
            .await
            .map_err(|e| {
                error!("HTTP request failed: {}", e);
                anyhow::anyhow!("Failed to send quote request to {}: {}", url, e)
            })?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Quote request failed: {}", error_text);
        }
        
        let quote: QuoteResponse = response.json()
            .await
            .context("Failed to parse quote response")?;
        
        info!(
            "Quote received: {} {} -> {} {}",
            quote.in_amount, input_mint,
            quote.out_amount, output_mint
        );
        
        Ok(quote)
    }
    
    pub async fn get_swap_transaction(
        &self,
        user_public_key: &Pubkey,
        quote: QuoteResponse,
    ) -> Result<SwapResponse> {
        let url = format!("{}/swap", self.api_url);
        
        info!("Requesting swap transaction from Jupiter API: {}", url);
        
        let swap_request = SwapRequest {
            quote_response: quote,
            user_public_key: user_public_key.to_string(),
            wrap_and_unwrap_sol: true,
            dynamic_compute_unit_limit: true,
            dynamic_slippage: true,
            prioritization_fee_lamports: Some(PrioritizationFee::Manual {
                priority_level_with_max_lamports: PriorityFeeLevel {
                    max_lamports: 1000000,
                    priority_level: "high".to_string(),
                },
            }),
            use_token_ledger: false,
        };
        
        let response = self.client
            .post(&url)
            .json(&swap_request)
            .send()
            .await
            .map_err(|e| {
                error!("HTTP request failed: {}", e);
                anyhow::anyhow!("Failed to send swap request to {}: {}", url, e)
            })?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Swap request failed: {}", error_text);
        }
        
        let response_text = response.text().await?;
        info!("Swap API response: {}", response_text);
        
        let swap: SwapResponse = serde_json::from_str(&response_text)
            .context("Failed to parse swap response")?;
        
        Ok(swap)
    }
    
    pub async fn execute_swap(
        &self,
        rpc_client: &RpcClient,
        wallet: &crate::wallet::Wallet,
        input_mint: &str,
        output_mint: &str,
        amount: u64,
        slippage_bps: u16,
    ) -> Result<String> {
        // Get quote
        let quote = self.get_quote(input_mint, output_mint, amount, slippage_bps).await?;
        
        // Get swap transaction
        let swap_response = self.get_swap_transaction(wallet.pubkey(), quote).await?;
        
        // Deserialize and sign transaction
        info!("Swap transaction base64 length: {}", swap_response.swap_transaction.len());
        
        let tx_bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &swap_response.swap_transaction)
            .context("Failed to decode transaction")?;
        
        info!("Transaction bytes length: {}", tx_bytes.len());
        
        // The new API may return either versioned or legacy transactions
        // We'll try to deserialize as versioned first, then fall back to legacy
        let (mut transaction, _is_versioned) = match bincode::deserialize::<VersionedTransaction>(&tx_bytes) {
            Ok(versioned_tx) => {
                info!("Successfully deserialized as versioned transaction");
                // For now, we'll try to convert to legacy for compatibility
                match versioned_tx.into_legacy_transaction() {
                    Some(legacy_tx) => (legacy_tx, false),
                    None => {
                        // If conversion fails, we need to handle versioned transactions
                        // For now, let's request legacy transactions instead
                        error!("Cannot convert versioned transaction to legacy format");
                        return Err(anyhow::anyhow!("Versioned transaction support not fully implemented. Please set as_legacy_transaction to true."));
                    }
                }
            }
            Err(_) => {
                // Try deserializing as legacy transaction as fallback
                match bincode::deserialize::<Transaction>(&tx_bytes) {
                    Ok(tx) => {
                        info!("Successfully deserialized as legacy transaction");
                        (tx, false)
                    }
                    Err(e) => {
                        error!("Failed to deserialize transaction");
                        error!("Deserialization error: {:?}", e);
                        return Err(anyhow::anyhow!("Failed to deserialize transaction: {}", e));
                    }
                }
            }
        };
        
        // Get recent blockhash
        let recent_blockhash = rpc_client.get_latest_blockhash()
            .context("Failed to get recent blockhash")?;
        
        transaction.message.recent_blockhash = recent_blockhash;
        
        // Sign transaction
        wallet.sign_transaction(&mut transaction)?;
        
        // Send and confirm transaction
        let signature = rpc_client
            .send_and_confirm_transaction(&transaction)
            .context("Failed to send and confirm transaction")?;
        
        info!("Swap executed successfully: {}", signature);
        
        Ok(signature.to_string())
    }
}

pub async fn get_price(
    jupiter_client: &JupiterClient,
    from_mint: &str,
    to_mint: &str,
    amount: u64,
) -> Result<f64> {
    let quote = jupiter_client.get_quote(from_mint, to_mint, amount, 0).await?;
    
    let in_amount = quote.in_amount.parse::<f64>()
        .context("Failed to parse input amount")?;
    let out_amount = quote.out_amount.parse::<f64>()
        .context("Failed to parse output amount")?;
    
    Ok(out_amount / in_amount)
}
