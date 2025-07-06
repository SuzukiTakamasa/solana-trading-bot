use anyhow::{Result, Context};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use solana_client::rpc_client::RpcClient;
use std::time::Duration;
use tokio::time::{sleep, timeout};
use tracing::{warn, debug, error};

pub struct Wallet {
    keypair: Keypair,
    pubkey: Pubkey,
}

impl Wallet {
    pub fn new(private_key: &str) -> Result<Self> {
        let decoded = bs58::decode(private_key)
            .into_vec()
            .context("Failed to decode private key")?;
        
        let keypair = Keypair::from_bytes(&decoded)
            .context("Failed to create keypair from private key")?;
        
        let pubkey = keypair.pubkey();
        
        Ok(Self { keypair, pubkey })
    }
    
    pub fn pubkey(&self) -> &Pubkey {
        &self.pubkey
    }
    
    pub fn sign_transaction(&self, transaction: &mut Transaction) -> Result<()> {
        transaction.sign(&[&self.keypair], transaction.message.recent_blockhash);
        Ok(())
    }
    
    pub async fn get_sol_balance(&self, client: &RpcClient) -> Result<f64> {
        const MAX_RETRIES: u32 = 3;
        const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
        let mut retry_delay = Duration::from_millis(500);
        
        for attempt in 0..MAX_RETRIES {
            match timeout(REQUEST_TIMEOUT, async {
                client.get_balance(&self.pubkey)
            }).await {
                Ok(Ok(balance)) => {
                    debug!("Successfully fetched SOL balance on attempt {}", attempt + 1);
                    return Ok(balance as f64 / 1_000_000_000.0); // Convert lamports to SOL
                }
                Ok(Err(e)) => {
                    let error_msg = format!("RPC error: {}", e);
                    if attempt < MAX_RETRIES - 1 {
                        warn!(
                            "Failed to get SOL balance (attempt {}/{}): {}. Retrying in {:?}...",
                            attempt + 1,
                            MAX_RETRIES,
                            error_msg,
                            retry_delay
                        );
                        sleep(retry_delay).await;
                        retry_delay *= 2; // Exponential backoff
                    } else {
                        error!("Failed to get SOL balance after {} attempts: {}", MAX_RETRIES, error_msg);
                        return Err(anyhow::anyhow!(
                            "Failed to get SOL balance after {} attempts: {}",
                            MAX_RETRIES,
                            error_msg
                        ));
                    }
                }
                Err(_) => {
                    let error_msg = "Request timeout";
                    if attempt < MAX_RETRIES - 1 {
                        warn!(
                            "Request timeout getting SOL balance (attempt {}/{}). Retrying in {:?}...",
                            attempt + 1,
                            MAX_RETRIES,
                            retry_delay
                        );
                        sleep(retry_delay).await;
                        retry_delay *= 2;
                    } else {
                        error!("Request timeout after {} attempts getting SOL balance: {}", MAX_RETRIES, error_msg);
                        return Err(anyhow::anyhow!(
                            "Request timeout after {} attempts getting SOL balance: {}", error_msg,
                            MAX_RETRIES
                        ));
                    }
                }
            }
        }
        
        unreachable!("Should have returned from the retry loop")
    }
    
    pub async fn get_token_balance(
        &self, 
        client: &RpcClient, 
        token_mint: &Pubkey
    ) -> Result<f64> {
        use spl_associated_token_account::get_associated_token_address;
        
        const MAX_RETRIES: u32 = 3;
        const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
        let mut retry_delay = Duration::from_millis(500);
        
        let token_account = get_associated_token_address(&self.pubkey, token_mint);
        
        for attempt in 0..MAX_RETRIES {
            match timeout(REQUEST_TIMEOUT, async {
                client.get_token_account_balance(&token_account)
            }).await {
                Ok(Ok(account_info)) => {
                    debug!("Successfully fetched token balance on attempt {}", attempt + 1);
                    return Ok(account_info.ui_amount.unwrap_or(0.0));
                }
                Ok(Err(e)) => {
                    let error_msg = format!("RPC error: {}", e);
                    if attempt < MAX_RETRIES - 1 {
                        warn!(
                            "Failed to get token balance (attempt {}/{}): {}. Retrying in {:?}...",
                            attempt + 1,
                            MAX_RETRIES,
                            error_msg,
                            retry_delay
                        );
                        sleep(retry_delay).await;
                        retry_delay *= 2;
                    } else {
                        error!("Failed to get token balance after {} attempts: {}", MAX_RETRIES, error_msg);
                        return Err(anyhow::anyhow!(
                            "Failed to get token balance after {} attempts: {}",
                            MAX_RETRIES,
                            error_msg
                        ));
                    }
                }
                Err(_) => {
                    let error_msg = "Request timeout";
                    if attempt < MAX_RETRIES - 1 {
                        warn!(
                            "Request timeout getting token balance (attempt {}/{}). Retrying in {:?}...",
                            attempt + 1,
                            MAX_RETRIES,
                            retry_delay
                        );
                        sleep(retry_delay).await;
                        retry_delay *= 2;
                    } else {
                        error!("Request timeout after {} attempts getting token balance: {}", MAX_RETRIES, error_msg);
                        return Err(anyhow::anyhow!(
                            "Request timeout after {} attempts getting token balance: {}", error_msg,
                            MAX_RETRIES
                        ));
                    }
                }
            }
        }
        
        unreachable!("Should have returned from the retry loop")
    }
}