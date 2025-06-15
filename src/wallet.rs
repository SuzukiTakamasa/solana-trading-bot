use anyhow::{Result, Context};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use solana_client::rpc_client::RpcClient;

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
        let balance = client.get_balance(&self.pubkey)
            .context("Failed to get SOL balance")?;
        
        Ok(balance as f64 / 1_000_000_000.0) // Convert lamports to SOL
    }
    
    pub async fn get_token_balance(
        &self, 
        client: &RpcClient, 
        token_mint: &Pubkey
    ) -> Result<f64> {
        use spl_associated_token_account::get_associated_token_address;
        
        let token_account = get_associated_token_address(&self.pubkey, token_mint);
        
        let account_info = client.get_token_account_balance(&token_account)
            .context("Failed to get token balance")?;
        
        Ok(account_info.ui_amount.unwrap_or(0.0))
    }
}