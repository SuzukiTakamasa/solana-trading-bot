use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use solana_sdk::{
    pubkey::Pubkey,
    transaction::{Transaction, VersionedTransaction},
};
use solana_client::rpc_client::RpcClient;
use tracing::{info, error};
use crate::service::retry_as_exponential_back_off;

#[derive(Debug, Serialize, Deserialize)]
pub struct QuoteRequest {
    #[serde(rename = "inputMint")]
    pub input_mint: String,
    #[serde(rename = "outputMint")]
    pub output_mint: String,
    pub amount: String,
    #[serde(rename = "slippageBps")]
    pub slippage_bps: u16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QuoteResponse {
    #[serde(rename = "inputMint")]
    pub input_mint: String,
    #[serde(rename = "inAmount")]
    pub in_amount: String,
    #[serde(rename = "outputMint")]
    pub output_mint: String,
    #[serde(rename = "outAmount")]
    pub out_amount: String,
    #[serde(rename = "otherAmountThreshold")]
    pub other_amount_threshold: String,
    #[serde(rename = "swapMode")]
    pub swap_mode: String,
    #[serde(rename = "slippageBps")]
    pub slippage_bps: u16,
    #[serde(rename = "priceImpactPct")]
    pub price_impact_pct: String,
    #[serde(rename = "routePlan")]
    pub route_plan: Vec<RoutePlanStep>,
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
    pub label: Option<String>,
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

#[derive(Debug, Serialize)]
pub struct SwapRequest {
    #[serde(rename = "userPublicKey")]
    pub user_public_key: String,
    #[serde(rename = "wrapAndUnwrapSol")]
    pub wrap_and_unwrap_sol: bool,
    #[serde(rename = "useSharedAccounts")]
    pub use_shared_accounts: bool,
    #[serde(rename = "feeAccount")]
    pub fee_account: Option<String>,
    #[serde(rename = "trackingAccount")]
    pub tracking_account: Option<String>,
    #[serde(rename = "computeUnitPriceMicroLamports")]
    pub compute_unit_price_micro_lamports: Option<u64>,
    #[serde(rename = "asLegacyTransaction")]
    pub as_legacy_transaction: bool,
    #[serde(rename = "useTokenLedger")]
    pub use_token_ledger: bool,
    #[serde(rename = "destinationTokenAccount")]
    pub destination_token_account: Option<String>,
    #[serde(rename = "dynamicComputeUnitLimit")]
    pub dynamic_compute_unit_limit: bool,
    #[serde(rename = "skipUserAccountsRpcCalls")]
    pub skip_user_accounts_rpc_calls: bool,
    #[serde(rename = "quoteResponse")]
    pub quote_response: QuoteResponse,
}

#[derive(Debug, Deserialize)]
pub struct SwapResponse {
    #[serde(rename = "swapTransaction")]
    pub swap_transaction: String,
}

pub struct JupiterClient {
    client: reqwest::Client,
    api_url: String,
}

impl JupiterClient {
    pub fn new(api_url: &str) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .connect_timeout(std::time::Duration::from_secs(10))
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .pool_max_idle_per_host(0)
            .user_agent("solana-trading-bot/1.0")
            .build()
            .expect("Failed to build HTTP client");
            
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
        
        info!(
            "Requesting quote: {} {} -> {} (slippage: {} bps)",
            amount, input_mint, output_mint, slippage_bps
        );
        
        let client = self.client.clone();
        let input_mint = input_mint.to_string();
        let output_mint = output_mint.to_string();
        let amount_str = amount.to_string();
        let slippage_str = slippage_bps.to_string();
        
        let response = retry_as_exponential_back_off(
            || async {
                client
                    .get(&url)
                    .query(&[
                        ("inputMint", &input_mint),
                        ("outputMint", &output_mint),
                        ("amount", &amount_str),
                        ("slippageBps", &slippage_str),
                    ])
                    .header("Accept", "application/json")
                    .send()
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to send quote request: {}", e))
            },
            "Quote request",
            3,
            500,
            None,
        )
        .await?;
        
        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unable to read error response".to_string());
            error!("Quote request failed with status {}: {}", status, error_text);
            anyhow::bail!("Quote request failed (status {}): {}", status, error_text)
        }
        
        let response_text = response.text().await
            .context("Failed to read response body")?;
            
        let quote: QuoteResponse = serde_json::from_str(&response_text)
            .map_err(|e| {
                error!("Failed to parse quote response: {}", e);
                error!("Response text: {}", response_text);
                anyhow::anyhow!("Failed to parse quote response: {}", e)
            })?;
        
        info!(
            "Quote received: {} {} -> {} {} (price impact: {}%)",
            quote.in_amount, input_mint,
            quote.out_amount, output_mint,
            quote.price_impact_pct
        );
        
        Ok(quote)
    }
    
    pub async fn get_swap_transaction(
        &self,
        user_public_key: &Pubkey,
        quote: QuoteResponse,
    ) -> Result<SwapResponse> {
        let url = format!("{}/swap", self.api_url);
        
        let swap_request = SwapRequest {
            user_public_key: user_public_key.to_string(),
            wrap_and_unwrap_sol: true,
            use_shared_accounts: true,
            fee_account: None,
            tracking_account: None,
            compute_unit_price_micro_lamports: Some(1000),
            as_legacy_transaction: true,
            use_token_ledger: false,
            destination_token_account: None,
            dynamic_compute_unit_limit: true,
            skip_user_accounts_rpc_calls: false,
            quote_response: quote,
        };
        
        let client = self.client.clone();
        
        let response = retry_as_exponential_back_off(
            || async {
                client
                    .post(&url)
                    .json(&swap_request)
                    .send()
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to send swap request: {}", e))
            },
            "Swap request",
            3,
            500,
            None,
        )
        .await?;
        
        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await?;
            error!("Swap request failed with status {}: {}", status, error_text);
            anyhow::bail!("Swap request failed: {}", error_text);
        }
        
        let response_text = response.text().await
            .context("Failed to read swap response body")?;
        
        let swap: SwapResponse = serde_json::from_str(&response_text)
            .map_err(|e| {
                error!("Failed to parse swap response: {}", e);
                error!("Response text: {}", response_text);
                anyhow::anyhow!("Failed to parse swap response: {}", e)
            })?;
        
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
        
        // Try to deserialize as versioned transaction first
        let (mut transaction, _is_versioned) = match bincode::deserialize::<VersionedTransaction>(&tx_bytes) {
            Ok(versioned_tx) => {
                info!("Successfully deserialized as versioned transaction");
                // Convert versioned transaction to legacy if possible
                match versioned_tx.into_legacy_transaction() {
                    Some(legacy_tx) => (legacy_tx, false),
                    None => {
                        // If we can't convert to legacy, try to handle it differently
                        error!("Cannot convert versioned transaction to legacy format");
                        return Err(anyhow::anyhow!("Jupiter returned a versioned transaction that cannot be converted to legacy format"));
                    }
                }
            }
            Err(_) => {
                // Try deserializing as legacy transaction
                match bincode::deserialize::<Transaction>(&tx_bytes) {
                    Ok(tx) => {
                        info!("Successfully deserialized as legacy transaction");
                        (tx, false)
                    }
                    Err(e) => {
                        error!("Failed to deserialize as both versioned and legacy transaction");
                        error!("Bincode deserialization error: {:?}", e);
                        error!("First 100 bytes of tx_bytes: {:?}", &tx_bytes[..tx_bytes.len().min(100)]);
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
        let signature = retry_as_exponential_back_off(
            || async {
                rpc_client
                    .send_and_confirm_transaction(&transaction)
                    .map_err(|e| anyhow::anyhow!("Failed to send and confirm transaction: {}", e))
            },
            "Send and confirm transaction",
            3,
            500,
            Some(std::time::Duration::from_secs(60)),
        )
        .await?;
        
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
    let quote = jupiter_client.get_quote(from_mint, to_mint, amount, 0).await
        .map_err(|e| {
            error!("Failed to get price quote for {} -> {}: {}", from_mint, to_mint, e);
            e
        })?;
    
    let in_amount = quote.in_amount.parse::<f64>()
        .context("Failed to parse input amount")?;
    let out_amount = quote.out_amount.parse::<f64>()
        .context("Failed to parse output amount")?;
    
    if in_amount == 0.0 {
        anyhow::bail!("Invalid input amount: 0");
    }
    
    Ok(out_amount / in_amount)
}
