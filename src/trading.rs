use anyhow::Result;
use chrono::{FixedOffset, TimeZone};
use chrono_tz::Asia::Tokyo;
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::Arc;
use tracing::{error, info};

use crate::{
    config::Config,
    firestore::{FirestoreDb, PriceHistory, TradingSession, ProfitTracking, generate_session_id, validate_price_data},
    jupiter::JupiterClient,
    wallet::Wallet,
};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Position {
    SOL,
    USDC,
}

impl std::fmt::Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Position::SOL => write!(f, "SOL"),
            Position::USDC => write!(f, "USDC"),
        }
    }
}

pub struct TradingState {
    pub position: Position,
    pub last_sol_price: Option<Decimal>,
    pub last_usdc_price: Option<Decimal>,
    pub last_trade_price: Option<Decimal>,
    pub total_profit_usdc: Decimal,
    pub total_trades: i64,
    pub winning_trades: i64,
    pub losing_trades: i64,
    pub firestore: Option<Arc<FirestoreDb>>,
}

impl TradingState {
    pub fn new() -> Self {
        Self {
            position: Position::USDC,
            last_sol_price: None,
            last_usdc_price: None,
            last_trade_price: None,
            total_profit_usdc: dec!(0),
            total_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            firestore: None,
        }
    }
    
    pub fn with_firestore(mut self, firestore: Arc<FirestoreDb>) -> Self {
        self.firestore = Some(firestore);
        self
    }
    
    pub async fn load_from_firestore(&mut self) -> Result<()> {
        if let Some(db) = &self.firestore {
            // Load position from latest trading session
            if let Ok(Some(latest_session)) = db.get_latest_trading_session().await {
                self.position = match latest_session.position_after.as_str() {
                    "SOL" => Position::SOL,
                    "USDC" => Position::USDC,
                    _ => Position::USDC, // Default to USDC if unknown
                };
                self.last_trade_price = Some(latest_session.price_at_trade);
                info!("Loaded position from latest trading session: {}, price: {}", self.position, latest_session.price_at_trade);
            }
            
            if let Ok(Some(latest_profit)) = db.get_latest_profit_tracking().await {
                self.total_profit_usdc = latest_profit.cumulative_profit_usdc;
                self.total_trades = latest_profit.total_trades;
                self.winning_trades = latest_profit.winning_trades;
                self.losing_trades = latest_profit.losing_trades;
                info!("Loaded trading state from Firestore: {} trades, {} USDC profit", 
                    self.total_trades, self.total_profit_usdc);
            }
            
            if let Ok(Some(latest_price)) = db.get_latest_price().await {
                self.last_sol_price = Some(latest_price.sol_price_usdc);
                self.last_usdc_price = Some(latest_price.usdc_price_sol);
            }
        }
        Ok(())
    }
}

/*
pub async fn perform_initial_swap(wallet: &Wallet, config: &Config) -> Result<()> {
    let rpc_client = RpcClient::new(&config.rpc_url);
    let jupiter_client = JupiterClient::new(&config.jupiter_api_url);
    
    // Get SOL balance
    let sol_balance = wallet.get_sol_balance(&rpc_client).await?;
    info!("Current SOL balance: {} SOL", sol_balance);
    
    // Keep some SOL for transaction fees (0.01 SOL)
    let sol_to_swap = sol_balance - 0.01;
    if sol_to_swap <= 0.0 {
        anyhow::bail!("Insufficient SOL balance for initial swap");
    }
    
    // Convert to lamports
    let amount_lamports = (sol_to_swap * 1_000_000_000.0) as u64;
    
    // Execute swap SOL -> USDC
    let signature = jupiter_client.execute_swap(
        &rpc_client,
        wallet,
        &config.sol_mint,
        &config.usdc_mint,
        amount_lamports,
        config.slippage_bps,
    ).await?;
    
    info!("Initial swap completed: {}", signature);
    
    Ok(())
}
*/

pub async fn check_and_trade(
    wallet: &Wallet,
    config: &Config,
    state: &mut TradingState,
) -> Result<Option<Decimal>> {
    let rpc_client = RpcClient::new(&config.rpc_url);
    let jupiter_client = JupiterClient::new(&config.jupiter_api_url);
    
    // Get current prices
    let (sol_price_in_usdc, usdc_price_in_sol) = get_current_prices(&jupiter_client, config).await?;
    
    info!("Current prices - SOL/USDC: {}, USDC/SOL: {}", sol_price_in_usdc, usdc_price_in_sol);
    
    // Validate prices
    validate_price_data(sol_price_in_usdc)?;
    validate_price_data(usdc_price_in_sol)?;
    
    // Store price history in Firestore
    if let Some(db) = &state.firestore {
        let price_history = PriceHistory {
            id: generate_session_id(),
            timestamp: Tokyo.from_utc_datetime(&chrono::Utc::now().naive_utc()).with_timezone(&FixedOffset::east_opt(9 * 3600).unwrap()),
            sol_price_usdc: sol_price_in_usdc,
            usdc_price_sol: usdc_price_in_sol,
            data_source: "Jupiter".to_string(),
            trading_session_id: generate_session_id(),
        };
        
        if let Err(e) = db.store_price_history(&price_history).await {
            error!("Failed to store price history: {}", e);
        }
    }
    
    let mut profit: Option<Decimal> = None;
    let trading_session_id = generate_session_id();
    
    // Get current balances before trade
    let sol_balance_before = wallet.get_sol_balance(&rpc_client).await?;
    let usdc_mint = Pubkey::from_str(&config.usdc_mint)?;
    let usdc_balance_before = wallet.get_token_balance(&rpc_client, &usdc_mint).await?;
    
    // Fetch price trend data
    let should_trade = if let Some(db) = &state.firestore {
        match db.get_price_trend(sol_price_in_usdc).await {
            Ok(trend) => {
                info!("Price trend - 1h: {:?}, 24h: {:?}, 7d: {:?}", 
                    trend.trend_1h, trend.trend_24h, trend.trend_7d);
                
                // Enhanced trading logic based on price trends
                should_make_trade(&state.position, &trend, sol_price_in_usdc, usdc_price_in_sol, state)
            }
            Err(e) => {
                error!("Failed to get price trend: {}", e);
                // Fallback to simple logic
                should_make_trade_simple(&state.position, sol_price_in_usdc, usdc_price_in_sol, state)
            }
        }
    } else {
        should_make_trade_simple(&state.position, sol_price_in_usdc, usdc_price_in_sol, state)
    };
    
    if !should_trade {
        return Ok(None);
    }
    
    match state.position {
        Position::USDC => {
            info!("Executing swap USDC -> SOL");
            
            if usdc_balance_before > 0.0 {
                // Convert USDC amount to smallest unit (6 decimals for USDC)
                let amount = (usdc_balance_before * 1_000_000.0) as u64;
                
                let signature = jupiter_client.execute_swap(
                    &rpc_client,
                    wallet,
                    &config.usdc_mint,
                    &config.sol_mint,
                    amount,
                    config.slippage_bps,
                ).await?;
                
                info!("Swap completed: {}", signature);
                
                // Get balances after trade
                let sol_balance_after = wallet.get_sol_balance(&rpc_client).await?;
                let usdc_balance_after = wallet.get_token_balance(&rpc_client, &usdc_mint).await?;
                
                // Calculate profit/loss
                let sol_gained = sol_balance_after - sol_balance_before;
                let usdc_spent = usdc_balance_before - usdc_balance_after;
                let effective_price = if sol_gained > 0.0 { usdc_spent / sol_gained } else { 0.0 };

                let profit_loss = if let Some(last_price) = state.last_sol_price {
                    let profit_per_sol = Decimal::from_f64_retain(effective_price).unwrap_or(dec!(0)) - last_price;
                    let total_profit = profit_per_sol * Decimal::from_f64_retain(sol_gained).unwrap_or(dec!(0));
                    state.total_profit_usdc += total_profit;
                    
                    match total_profit.cmp(&dec!(0)) {
                        std::cmp::Ordering::Greater => state.winning_trades += 1,
                        std::cmp::Ordering::Less => state.losing_trades += 1,
                        std::cmp::Ordering::Equal => {}
                    }
                    
                    Some(total_profit)
                } else {
                    None
                };

                profit = profit_loss;
                
                // Store trading session
                if let Some(db) = &state.firestore {
                    let session = TradingSession {
                        id: trading_session_id.clone(),
                        timestamp: Tokyo.from_utc_datetime(&chrono::Utc::now().naive_utc()).with_timezone(&FixedOffset::east_opt(9 * 3600).unwrap()),
                        position_before: "USDC".to_string(),
                        position_after: "SOL".to_string(),
                        action: "BUY_SOL".to_string(),
                        sol_balance_before: Decimal::from_f64_retain(sol_balance_before).unwrap_or(dec!(0)),
                        usdc_balance_before: Decimal::from_f64_retain(usdc_balance_before).unwrap_or(dec!(0)),
                        sol_balance_after: Decimal::from_f64_retain(sol_balance_after).unwrap_or(dec!(0)),
                        usdc_balance_after: Decimal::from_f64_retain(usdc_balance_after).unwrap_or(dec!(0)),
                        price_at_trade: sol_price_in_usdc,
                        slippage: Some(Decimal::from_f64_retain(effective_price).unwrap_or(dec!(0)) - sol_price_in_usdc),
                        gas_fee: None,
                        profit_loss,
                        cumulative_profit: Some(state.total_profit_usdc),
                    };
                    
                    if let Err(e) = db.store_trading_session(&session).await {
                        error!("Failed to store trading session: {}", e);
                    }
                }
                
                // Update last trade price and position
                state.total_profit_usdc = profit_loss.unwrap_or(dec!(0));
                state.position = Position::SOL;
            }
        }
        Position::SOL => {
            info!("Executing swap SOL -> USDC");
            
            // Keep some SOL for fees
            let sol_to_swap = sol_balance_before - 0.01;
            if sol_to_swap > 0.0 {
                let amount_lamports = (sol_to_swap * 1_000_000_000.0) as u64;
                
                let signature = jupiter_client.execute_swap(
                    &rpc_client,
                    wallet,
                    &config.sol_mint,
                    &config.usdc_mint,
                    amount_lamports,
                    config.slippage_bps,
                ).await?;
                
                info!("Swap completed: {}", signature);
                
                // Get balances after trade
                let sol_balance_after = wallet.get_sol_balance(&rpc_client).await?;
                let usdc_balance_after = wallet.get_token_balance(&rpc_client, &usdc_mint).await?;
                
                // Calculate profit/loss
                let usdc_gained = usdc_balance_after - usdc_balance_before;
                let sol_spent = sol_balance_before - sol_balance_after;
                let effective_price = if sol_spent > 0.0 { usdc_gained / sol_spent } else { 0.0 };
                
                // Calculate profit if we have a previous price
                let profit_loss = if let Some(last_price) = state.last_sol_price {
                    let profit_per_sol = sol_price_in_usdc - last_price;
                    let total_profit = profit_per_sol * Decimal::from_f64_retain(sol_spent).unwrap_or(dec!(0));
                    state.total_profit_usdc += total_profit;
                    

                    match total_profit.cmp(&dec!(0)) {
                        std::cmp::Ordering::Greater => state.winning_trades += 1,
                        std::cmp::Ordering::Less => state.losing_trades += 1,
                        std::cmp::Ordering::Equal => {}
                    }
                    
                    Some(total_profit)
                } else {
                    None
                };
                
                profit = profit_loss;
                
                // Store trading session
                if let Some(db) = &state.firestore {
                    let session = TradingSession {
                        id: trading_session_id.clone(),
                        timestamp: Tokyo.from_utc_datetime(&chrono::Utc::now().naive_utc()).with_timezone(&FixedOffset::east_opt(9 * 3600).unwrap()),
                        position_before: "SOL".to_string(),
                        position_after: "USDC".to_string(),
                        action: "SELL_SOL".to_string(),
                        sol_balance_before: Decimal::from_f64_retain(sol_balance_before).unwrap_or(dec!(0)),
                        usdc_balance_before: Decimal::from_f64_retain(usdc_balance_before).unwrap_or(dec!(0)),
                        sol_balance_after: Decimal::from_f64_retain(sol_balance_after).unwrap_or(dec!(0)),
                        usdc_balance_after: Decimal::from_f64_retain(usdc_balance_after).unwrap_or(dec!(0)),
                        price_at_trade: sol_price_in_usdc,
                        slippage: Some(Decimal::from_f64_retain(effective_price).unwrap_or(dec!(0)) - sol_price_in_usdc),
                        gas_fee: None,
                        profit_loss,
                        cumulative_profit: Some(state.total_profit_usdc),
                    };
                    
                    if let Err(e) = db.store_trading_session(&session).await {
                        error!("Failed to store trading session: {}", e);
                    }
                    
                    // Update profit tracking
                    if profit_loss.is_some() {
                        let profit_tracking = ProfitTracking {
                            id: generate_session_id(),
                            timestamp: Tokyo.from_utc_datetime(&chrono::Utc::now().naive_utc()).with_timezone(&FixedOffset::east_opt(9 * 3600).unwrap()),
                            trading_session_id,
                            profit_loss_usdc: profit_loss.unwrap_or(dec!(0)),
                            cumulative_profit_usdc: state.total_profit_usdc,
                            roi_percentage: if usdc_balance_before > 0.0 {
                                (state.total_profit_usdc / Decimal::from_f64_retain(usdc_balance_before).unwrap_or(dec!(1))) * dec!(100)
                            } else {
                                dec!(0)
                            },
                            total_trades: state.total_trades + 1,
                            winning_trades: state.winning_trades,
                            losing_trades: state.losing_trades,
                        };
                        
                        if let Err(e) = db.store_profit_tracking(&profit_tracking).await {
                            error!("Failed to store profit tracking: {}", e);
                        }
                    }
                }
                
                // Update last trade price and position
                state.total_profit_usdc = profit_loss.unwrap_or(dec!(0));
                state.position = Position::USDC;
            }
        }
    }
    
    Ok(profit)
}

async fn get_current_prices(
    jupiter_client: &JupiterClient,
    config: &Config,
) -> Result<(Decimal, Decimal)> {
    // Get SOL price in USDC (1 SOL = ? USDC)
    let sol_price = crate::jupiter::get_price(
        jupiter_client,
        &config.sol_mint,
        &config.usdc_mint,
        1_000_000_000, // 1 SOL in lamports
    ).await?;
    
    // Get USDC price in SOL (1 USDC = ? SOL)
    let usdc_price = crate::jupiter::get_price(
        jupiter_client,
        &config.usdc_mint,
        &config.sol_mint,
        1_000_000, // 1 USDC in smallest unit
    ).await?;
    
    // Adjust for decimal places
    let sol_price_adjusted = sol_price / 1_000_000.0; // USDC has 6 decimals
    let usdc_price_adjusted = usdc_price / 1_000_000_000.0; // SOL has 9 decimals
    
    Ok((
        Decimal::from_f64_retain(sol_price_adjusted).unwrap_or(dec!(0)),
        Decimal::from_f64_retain(usdc_price_adjusted).unwrap_or(dec!(0)),
    ))
}

// Enhanced trading logic with trend analysis
fn should_make_trade(
    position: &Position,
    _trend: &crate::firestore::PriceTrend,
    sol_price: Decimal,
    _usdc_price: Decimal,
    state: &TradingState,
) -> bool {
    match position {
        Position::USDC => {
            // Buy SOL if the price has increased by 1% or more compared to the price from the last trade
            state.last_trade_price
                .map(|last_price| sol_price >= last_price * dec!(1.01))
                .unwrap_or(false)
        }
        Position::SOL => {
            // Sell SOL if the price has increased by 1% or more compared to the price from the last trade
            state.last_trade_price
                .map(|last_price| sol_price >= last_price * dec!(1.01))
                .unwrap_or(false)
        }
    }
}

// Simple trading logic fallback
fn should_make_trade_simple(
    position: &Position,
    sol_price: Decimal,
    _usdc_price: Decimal,
    state: &TradingState,
) -> bool {
    match position {
        Position::USDC => {
            // Buy SOL if the price has increased by 1% or more from last trade
            state.last_trade_price
                .map(|last| sol_price >= last * dec!(1.01))
                .unwrap_or(false)
        }
        Position::SOL => {
            // Sell SOL if the price has increased by 1% or more from last trade
            state.last_trade_price
                .map(|last| sol_price >= last * dec!(1.01))
                .unwrap_or(false)
        }
    }
}

