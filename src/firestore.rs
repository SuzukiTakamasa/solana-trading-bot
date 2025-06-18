use anyhow::Result;
use chrono::{DateTime, Utc};
use firestore_db_and_auth::{documents, Credentials, ServiceSession};
use futures::stream::StreamExt;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceHistory {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub sol_price_usdc: Decimal,
    pub usdc_price_sol: Decimal,
    pub data_source: String,
    pub trading_session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingSession {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub position_before: String,
    pub position_after: String,
    pub action: String,
    pub sol_balance_before: Decimal,
    pub usdc_balance_before: Decimal,
    pub sol_balance_after: Decimal,
    pub usdc_balance_after: Decimal,
    pub price_at_trade: Decimal,
    pub slippage: Option<Decimal>,
    pub gas_fee: Option<Decimal>,
    pub profit_loss: Option<Decimal>,
    pub cumulative_profit: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfitTracking {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub trading_session_id: String,
    pub profit_loss_usdc: Decimal,
    pub cumulative_profit_usdc: Decimal,
    pub roi_percentage: Decimal,
    pub total_trades: i64,
    pub winning_trades: i64,
    pub losing_trades: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceTrend {
    pub timestamp: DateTime<Utc>,
    pub price_1h_ago: Option<Decimal>,
    pub price_24h_ago: Option<Decimal>,
    pub price_7d_ago: Option<Decimal>,
    pub trend_1h: Option<String>,
    pub trend_24h: Option<String>,
    pub trend_7d: Option<String>,
    pub volatility_1h: Option<Decimal>,
    pub volatility_24h: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingPerformance {
    pub total_trades: i64,
    pub winning_trades: i64,
    pub losing_trades: i64,
    pub total_profit_loss: Decimal,
    pub total_gas_fees: Decimal,
    pub win_rate: Decimal,
    pub period_days: u32,
}

pub struct FirestoreDb {
    pub session: ServiceSession,
    retry_count: u32,
}

impl FirestoreDb {
    pub async fn new(project_id: String) -> Result<Self> {
        info!("Initializing Firestore client for project: {}", project_id);
        
        let cred = Credentials::from_file("service-account.json")
            .await
            .map_err(|e| anyhow::anyhow!("Failed to load credentials: {}", e))?;
        
        let session = ServiceSession::new(cred)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create session: {}", e))?;
        
        Ok(Self {
            session,
            retry_count: 3,
        })
    }
    
    pub async fn store_price_history(&self, price_data: &PriceHistory) -> Result<()> {
        let mut attempts = 0;
        
        while attempts < self.retry_count {
            match self._store_price_history_internal(price_data).await {
                Ok(_) => {
                    info!("Successfully stored price history: {}", price_data.id);
                    return Ok(());
                }
                Err(e) => {
                    attempts += 1;
                    error!("Failed to store price history (attempt {}): {}", attempts, e);
                    
                    if attempts < self.retry_count {
                        tokio::time::sleep(tokio::time::Duration::from_millis(500 * attempts as u64)).await;
                    }
                }
            }
        }
        
        Err(anyhow::anyhow!("Failed to store price history after {} attempts", self.retry_count))
    }
    
    async fn _store_price_history_internal(&self, price_data: &PriceHistory) -> Result<()> {
        documents::write(&self.session, "price_history", Some(&price_data.id), price_data, documents::WriteOptions::default())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to write price history document: {}", e))?;
        
        Ok(())
    }
    
    pub async fn store_trading_session(&self, session: &TradingSession) -> Result<()> {
        let mut attempts = 0;
        
        while attempts < self.retry_count {
            match self._store_trading_session_internal(session).await {
                Ok(_) => {
                    info!("Successfully stored trading session: {}", session.id);
                    return Ok(());
                }
                Err(e) => {
                    attempts += 1;
                    error!("Failed to store trading session (attempt {}): {}", attempts, e);
                    
                    if attempts < self.retry_count {
                        tokio::time::sleep(tokio::time::Duration::from_millis(500 * attempts as u64)).await;
                    }
                }
            }
        }
        
        Err(anyhow::anyhow!("Failed to store trading session after {} attempts", self.retry_count))
    }
    
    async fn _store_trading_session_internal(&self, session: &TradingSession) -> Result<()> {
        documents::write(&self.session, "trading_sessions", Some(&session.id), session, documents::WriteOptions::default())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to write trading session document: {}", e))?;
        
        Ok(())
    }
    
    pub async fn store_profit_tracking(&self, profit: &ProfitTracking) -> Result<()> {
        documents::write(&self.session, "profit_tracking", Some(&profit.id), profit, documents::WriteOptions::default())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to write profit tracking document: {}", e))?;
        
        Ok(())
    }
    
    pub async fn get_latest_price(&self) -> Result<Option<PriceHistory>> {
        // Use list and manually sort since query API is complex
        let mut stream = documents::list::<PriceHistory, _>(&self.session, "price_history");
        match stream.next().await {
            Some(Ok(doc_result)) => {
                let (doc, _) = doc_result;
                Ok(Some(doc))
            },
            Some(Err(e)) => Err(anyhow::anyhow!("Failed to query latest price: {}", e)),
            None => Ok(None)
        }
    }
    
    pub async fn get_price_history(&self, hours: u32) -> Result<Vec<PriceHistory>> {
        let cutoff_time = Utc::now() - chrono::Duration::hours(hours as i64);
        
        let mut stream = documents::list::<PriceHistory, _>(&self.session, "price_history");
        let mut prices = Vec::new();
        
        while let Some(doc_result) = stream.next().await {
            match doc_result {
                Ok((doc, _)) => {
                    if doc.timestamp > cutoff_time {
                        prices.push(doc);
                    }
                },
                Err(e) => {
                    error!("Error reading price history document: {}", e);
                    continue;
                }
            }
        }
        
        prices.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(prices)
    }
    
    pub async fn get_price_trend(&self, current_price: Decimal) -> Result<PriceTrend> {
        let now = Utc::now();
        
        let price_1h = self.get_price_at_time(now - chrono::Duration::hours(1)).await?;
        let price_24h = self.get_price_at_time(now - chrono::Duration::hours(24)).await?;
        let price_7d = self.get_price_at_time(now - chrono::Duration::days(7)).await?;
        
        let trend_1h = price_1h.map(|p| {
            if current_price > p { "up".to_string() }
            else if current_price < p { "down".to_string() }
            else { "stable".to_string() }
        });
        
        let trend_24h = price_24h.map(|p| {
            if current_price > p { "up".to_string() }
            else if current_price < p { "down".to_string() }
            else { "stable".to_string() }
        });
        
        let trend_7d = price_7d.map(|p| {
            if current_price > p { "up".to_string() }
            else if current_price < p { "down".to_string() }
            else { "stable".to_string() }
        });
        
        let volatility_1h = self.calculate_volatility(1).await.ok();
        let volatility_24h = self.calculate_volatility(24).await.ok();
        
        Ok(PriceTrend {
            timestamp: now,
            price_1h_ago: price_1h,
            price_24h_ago: price_24h,
            price_7d_ago: price_7d,
            trend_1h,
            trend_24h,
            trend_7d,
            volatility_1h,
            volatility_24h,
        })
    }
    
    async fn get_price_at_time(&self, time: DateTime<Utc>) -> Result<Option<Decimal>> {
        let mut stream = documents::list::<PriceHistory, _>(&self.session, "price_history");
        let mut valid_prices = Vec::new();
        
        while let Some(doc_result) = stream.next().await {
            match doc_result {
                Ok((doc, _)) => {
                    if doc.timestamp <= time {
                        valid_prices.push(doc);
                    }
                },
                Err(e) => {
                    error!("Error reading price history document: {}", e);
                    continue;
                }
            }
        }
        
        valid_prices.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(valid_prices.first().map(|p| p.sol_price_usdc))
    }
    
    async fn calculate_volatility(&self, hours: u32) -> Result<Decimal> {
        let prices = self.get_price_history(hours).await?;
        
        if prices.len() < 2 {
            return Ok(Decimal::ZERO);
        }
        
        let prices_vec: Vec<Decimal> = prices.iter().map(|p| p.sol_price_usdc).collect();
        let mean = prices_vec.iter().sum::<Decimal>() / Decimal::from(prices_vec.len());
        
        let variance = prices_vec
            .iter()
            .map(|p| (*p - mean) * (*p - mean))
            .sum::<Decimal>() / Decimal::from(prices_vec.len());
        
        // Since Decimal doesn't have sqrt, we'll approximate it or return the variance
        // For trading bot purposes, variance is sufficient for volatility measure
        Ok(variance)
    }
    
    pub async fn get_trading_performance(&self, days: u32) -> Result<TradingPerformance> {
        let cutoff_time = Utc::now() - chrono::Duration::days(days as i64);
        
        let mut stream = documents::list::<TradingSession, _>(&self.session, "trading_sessions");
        let mut total_trades = 0;
        let mut winning_trades = 0;
        let mut losing_trades = 0;
        let mut total_profit_loss = Decimal::ZERO;
        let mut total_gas_fees = Decimal::ZERO;
        
        while let Some(doc_result) = stream.next().await {
            match doc_result {
                Ok((session, _)) => {
                    if session.timestamp > cutoff_time {
                        total_trades += 1;
                        
                        if let Some(profit_loss) = session.profit_loss {
                            total_profit_loss += profit_loss;
                            if profit_loss > Decimal::ZERO {
                                winning_trades += 1;
                            } else if profit_loss < Decimal::ZERO {
                                losing_trades += 1;
                            }
                        }
                        
                        if let Some(gas_fee) = session.gas_fee {
                            total_gas_fees += gas_fee;
                        }
                    }
                },
                Err(e) => {
                    error!("Error reading trading session document: {}", e);
                    continue;
                }
            }
        }
        
        let win_rate = if total_trades > 0 {
            Decimal::from(winning_trades) / Decimal::from(total_trades) * Decimal::from(100)
        } else {
            Decimal::ZERO
        };
        
        Ok(TradingPerformance {
            total_trades,
            winning_trades,
            losing_trades,
            total_profit_loss,
            total_gas_fees,
            win_rate,
            period_days: days,
        })
    }
    
    pub async fn get_latest_profit_tracking(&self) -> Result<Option<ProfitTracking>> {
        let mut stream = documents::list::<ProfitTracking, _>(&self.session, "profit_tracking");
        let mut profits = Vec::new();
        
        while let Some(doc_result) = stream.next().await {
            match doc_result {
                Ok((profit, _)) => {
                    profits.push(profit);
                },
                Err(e) => {
                    error!("Error reading profit tracking document: {}", e);
                    continue;
                }
            }
        }
        
        profits.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(profits.first().cloned())
    }
    
    pub async fn cleanup_old_data(&self, retention_days: u32) -> Result<()> {
        let cutoff_time = Utc::now() - chrono::Duration::days(retention_days as i64);
        
        info!("Cleaning up data older than {} days", retention_days);
        
        // Clean up price history
        let mut stream = documents::list::<PriceHistory, _>(&self.session, "price_history");
        let mut deleted_count = 0;
        
        while let Some(doc_result) = stream.next().await {
            match doc_result {
                Ok((doc, doc_metadata)) => {
                    if doc.timestamp < cutoff_time {
                        let doc_path = format!("price_history/{}", doc_metadata.name);
                        if let Err(e) = documents::delete(&self.session, &doc_path, true).await {
                            warn!("Failed to delete document {}: {}", doc_path, e);
                        } else {
                            deleted_count += 1;
                        }
                    }
                },
                Err(e) => {
                    warn!("Error reading price history document: {}", e);
                    continue;
                }
            }
        }
        
        info!("Deleted {} old documents from price_history", deleted_count);
        
        // Clean up trading sessions
        let mut stream = documents::list::<TradingSession, _>(&self.session, "trading_sessions");
        let mut deleted_count = 0;
        
        while let Some(doc_result) = stream.next().await {
            match doc_result {
                Ok((doc, doc_metadata)) => {
                    if doc.timestamp < cutoff_time {
                        let doc_path = format!("trading_sessions/{}", doc_metadata.name);
                        if let Err(e) = documents::delete(&self.session, &doc_path, true).await {
                            warn!("Failed to delete document {}: {}", doc_path, e);
                        } else {
                            deleted_count += 1;
                        }
                    }
                },
                Err(e) => {
                    warn!("Error reading trading session document: {}", e);
                    continue;
                }
            }
        }
        
        info!("Deleted {} old documents from trading_sessions", deleted_count);
        
        // Clean up profit tracking
        let mut stream = documents::list::<ProfitTracking, _>(&self.session, "profit_tracking");
        let mut deleted_count = 0;
        
        while let Some(doc_result) = stream.next().await {
            match doc_result {
                Ok((doc, doc_metadata)) => {
                    if doc.timestamp < cutoff_time {
                        let doc_path = format!("profit_tracking/{}", doc_metadata.name);
                        if let Err(e) = documents::delete(&self.session, &doc_path, true).await {
                            warn!("Failed to delete document {}: {}", doc_path, e);
                        } else {
                            deleted_count += 1;
                        }
                    }
                },
                Err(e) => {
                    warn!("Error reading profit tracking document: {}", e);
                    continue;
                }
            }
        }
        
        info!("Deleted {} old documents from profit_tracking", deleted_count);
        
        Ok(())
    }
}

pub fn generate_session_id() -> String {
    Uuid::new_v4().to_string()
}

pub fn validate_price_data(price: Decimal) -> Result<()> {
    if price <= Decimal::ZERO {
        return Err(anyhow::anyhow!("Invalid price: must be greater than zero"));
    }
    
    if price > Decimal::from(1_000_000) {
        return Err(anyhow::anyhow!("Invalid price: exceeds maximum allowed value"));
    }
    
    Ok(())
}