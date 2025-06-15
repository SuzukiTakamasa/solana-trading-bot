use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use solana_sdk::{
    pubkey::Pubkey,
    transaction::Transaction,
};
use solana_client::rpc_client::RpcClient;
use tracing::info;

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
    pub quote_response: QuoteResponse,
}

#[derive(Debug, Deserialize)]
pub struct SwapResponse {
    #[serde(rename = "swapTransaction")]
    pub swap_transaction: String,
    #[allow(dead_code)]
    #[serde(rename = "lastValidBlockHeight")]
    pub last_valid_block_height: u64,
    #[allow(dead_code)]
    #[serde(rename = "prioritizationFeeLamports")]
    pub prioritization_fee_lamports: Option<u64>,
}

pub struct JupiterClient {
    client: reqwest::Client,
    api_url: String,
}

impl JupiterClient {
    pub fn new(api_url: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
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
        
        let response = self.client
            .get(&url)
            .query(&[
                ("inputMint", input_mint),
                ("outputMint", output_mint),
                ("amount", &amount.to_string()),
                ("slippageBps", &slippage_bps.to_string()),
            ])
            .send()
            .await
            .context("Failed to send quote request")?;
        
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
        
        let swap_request = SwapRequest {
            user_public_key: user_public_key.to_string(),
            wrap_and_unwrap_sol: true,
            use_shared_accounts: true,
            fee_account: None,
            tracking_account: None,
            compute_unit_price_micro_lamports: Some(1000),
            as_legacy_transaction: false,
            use_token_ledger: false,
            destination_token_account: None,
            dynamic_compute_unit_limit: true,
            skip_user_accounts_rpc_calls: false,
            quote_response: quote,
        };
        
        let response = self.client
            .post(&url)
            .json(&swap_request)
            .send()
            .await
            .context("Failed to send swap request")?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Swap request failed: {}", error_text);
        }
        
        let swap: SwapResponse = response.json()
            .await
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
        let tx_bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &swap_response.swap_transaction)
            .context("Failed to decode transaction")?;
        
        let mut transaction: Transaction = bincode::deserialize(&tx_bytes)
            .context("Failed to deserialize transaction")?;
        
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