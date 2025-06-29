use anyhow::Result;
use chrono::{DateTime, FixedOffset, TimeZone};
use chrono_tz::Asia::Tokyo;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tracing::{error, info};
use uuid::Uuid;
use gcp_auth::{AuthenticationManager, CustomServiceAccount};
use reqwest::{Client, header::{AUTHORIZATION, CONTENT_TYPE}};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceHistory {
    pub id: String,
    pub timestamp: DateTime<FixedOffset>,
    pub sol_price_usdc: Decimal,
    pub usdc_price_sol: Decimal,
    pub data_source: String,
    pub trading_session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingSession {
    pub id: String,
    pub timestamp: DateTime<FixedOffset>,
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
    pub timestamp: DateTime<FixedOffset>,
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
    pub timestamp: DateTime<FixedOffset>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirestoreDocument {
    name: Option<String>,
    fields: HashMap<String, FirestoreValue>,
    create_time: Option<String>,
    update_time: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum FirestoreValue {
    StringValue { string_value: String },
    IntegerValue { integer_value: String },
    DoubleValue { double_value: f64 },
    BooleanValue { boolean_value: bool },
    TimestampValue { timestamp_value: String },
    NullValue { null_value: String },
    ArrayValue { array_value: FirestoreArrayValue },
    MapValue { map_value: FirestoreMapValue },
    Other(serde_json::Value),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FirestoreArrayValue {
    values: Vec<FirestoreValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FirestoreMapValue {
    fields: HashMap<String, FirestoreValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ListDocumentsResponse {
    documents: Option<Vec<FirestoreDocument>>,
    next_page_token: Option<String>,
}

pub struct FirestoreDb {
    client: Client,
    auth_manager: AuthenticationManager,
    pub project_id: String,
    pub database_id: String,
    retry_count: u32,
}

impl FirestoreDb {
    pub async fn new(project_id: String) -> Result<Self> {
        info!("Initializing Firestore client for project: {}", project_id);
        
        let client = Client::new();
        
        // Setup authentication
        let auth_manager = if let Ok(json_path) = std::env::var("CLOUD_RUN_CREDENTIALS") {
            info!("Using credentials files: {}", json_path);
            let service_account = CustomServiceAccount::from_file(&json_path)?;
            AuthenticationManager::from(service_account)
        } else {
            info!("Using default credentials (Cloud Run service account)");
            AuthenticationManager::new().await?
        };
        
        Ok(Self {
            client,
            auth_manager,
            project_id,
            database_id: "(default)".to_string(),
            retry_count: 3,
        })
    }
    
    fn get_document_url(&self, collection: &str, document_id: &str) -> String {
        format!(
            "https://firestore.googleapis.com/v1/projects/{}/databases/{}/documents/{}/{}",
            self.project_id, self.database_id, collection, document_id
        )
    }
    
    fn get_collection_url(&self, collection: &str) -> String {
        format!(
            "https://firestore.googleapis.com/v1/projects/{}/databases/{}/documents/{}",
            self.project_id, self.database_id, collection
        )
    }
    
    pub async fn get_auth_token(&self) -> Result<String> {
        let token = self.auth_manager
            .get_token(&["https://www.googleapis.com/auth/datastore"])
            .await?;
        Ok(format!("Bearer {}", token.as_str()))
    }
    
    fn serialize_to_firestore_document<T: Serialize>(&self, data: &T) -> Result<FirestoreDocument> {
        let json_value = serde_json::to_value(data)?;
        let fields = self.json_to_firestore_fields(json_value)?;
        
        Ok(FirestoreDocument {
            fields,
            name: None,
            create_time: None,
            update_time: None,
        })
    }
    
    fn json_to_firestore_fields(&self, value: JsonValue) -> Result<HashMap<String, FirestoreValue>> {
        match value {
            JsonValue::Object(map) => {
                let mut fields = HashMap::new();
                for (key, val) in map {
                    fields.insert(key, self.json_to_firestore_value(val)?);
                }
                Ok(fields)
            }
            _ => Err(anyhow::anyhow!("Expected JSON object")),
        }
    }
    
    fn json_to_firestore_value(&self, value: JsonValue) -> Result<FirestoreValue> {
        Ok(match value {
            JsonValue::Null => FirestoreValue::NullValue { null_value: "NULL_VALUE".to_string() },
            JsonValue::Bool(b) => FirestoreValue::BooleanValue { boolean_value: b },
            JsonValue::Number(n) => {
                if let Some(i) = n.as_i64() {
                    FirestoreValue::IntegerValue { integer_value: i.to_string() }
                } else if let Some(f) = n.as_f64() {
                    FirestoreValue::DoubleValue { double_value: f }
                } else {
                    // Handle Decimal as string
                    FirestoreValue::StringValue { string_value: n.to_string() }
                }
            },
            JsonValue::String(s) => {
                // Check if it's a timestamp
                if s.ends_with('Z') && s.contains('T') {
                    FirestoreValue::TimestampValue { timestamp_value: s }
                } else {
                    FirestoreValue::StringValue { string_value: s }
                }
            },
            JsonValue::Array(arr) => {
                let values = arr.into_iter()
                    .map(|v| self.json_to_firestore_value(v))
                    .collect::<Result<Vec<_>>>()?;
                FirestoreValue::ArrayValue { array_value: FirestoreArrayValue { values } }
            },
            JsonValue::Object(map) => {
                let fields = self.json_to_firestore_fields(JsonValue::Object(map))?;
                FirestoreValue::MapValue { map_value: FirestoreMapValue { fields } }
            },
        })
    }
    
    pub fn firestore_document_to_json<T: for<'de> Deserialize<'de>>(&self, doc: FirestoreDocument) -> Result<T> {
        let json_value = self.firestore_fields_to_json(doc.fields)?;
        serde_json::from_value(json_value).map_err(|e| anyhow::anyhow!("Failed to deserialize: {}", e))
    }
    
    fn firestore_fields_to_json(&self, fields: HashMap<String, FirestoreValue>) -> Result<JsonValue> {
        let mut map = serde_json::Map::new();
        for (key, value) in fields {
            map.insert(key, self.firestore_value_to_json(value)?);
        }
        Ok(JsonValue::Object(map))
    }
    
    fn firestore_value_to_json(&self, value: FirestoreValue) -> Result<JsonValue> {
        Ok(match value {
            FirestoreValue::NullValue { .. } => JsonValue::Null,
            FirestoreValue::BooleanValue { boolean_value } => JsonValue::Bool(boolean_value),
            FirestoreValue::IntegerValue { integer_value } => {
                JsonValue::Number(integer_value.parse::<i64>()?.into())
            },
            FirestoreValue::DoubleValue { double_value } => {
                JsonValue::Number(serde_json::Number::from_f64(double_value)
                    .ok_or_else(|| anyhow::anyhow!("Invalid float value"))?)
            },
            FirestoreValue::StringValue { string_value } => JsonValue::String(string_value),
            FirestoreValue::TimestampValue { timestamp_value } => JsonValue::String(timestamp_value),
            FirestoreValue::ArrayValue { array_value } => {
                let values = array_value.values.into_iter()
                    .map(|v| self.firestore_value_to_json(v))
                    .collect::<Result<Vec<_>>>()?;
                JsonValue::Array(values)
            },
            FirestoreValue::MapValue { map_value } => {
                self.firestore_fields_to_json(map_value.fields)?
            },
            FirestoreValue::Other(_) => {
                serde_json::Value::Object(serde_json::Map::new()) // Handle as empty object for now
            },
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
        let document = self.serialize_to_firestore_document(price_data)?;
        let url = self.get_document_url("price_history", &price_data.id);
        let auth_token = self.get_auth_token().await?;
        
        self.client
            .patch(&url)
            .header(AUTHORIZATION, auth_token)
            .header(CONTENT_TYPE, "application/json")
            .json(&document)
            .send()
            .await?
            .error_for_status()?;
        
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
        let document = self.serialize_to_firestore_document(session)?;
        let url = self.get_document_url("trading_sessions", &session.id);
        let auth_token = self.get_auth_token().await?;
        
        self.client
            .patch(&url)
            .header(AUTHORIZATION, auth_token)
            .header(CONTENT_TYPE, "application/json")
            .json(&document)
            .send()
            .await?
            .error_for_status()?;
        
        Ok(())
    }
    
    pub async fn store_profit_tracking(&self, profit: &ProfitTracking) -> Result<()> {
        let document = self.serialize_to_firestore_document(profit)?;
        let url = self.get_document_url("profit_tracking", &profit.id);
        let auth_token = self.get_auth_token().await?;
        
        self.client
            .patch(&url)
            .header(AUTHORIZATION, auth_token)
            .header(CONTENT_TYPE, "application/json")
            .json(&document)
            .send()
            .await?
            .error_for_status()?;
        
        Ok(())
    }
    
    pub async fn get_latest_price(&self) -> Result<Option<PriceHistory>> {
        let url = format!("{}{}", self.get_collection_url("price_history"), "?pageSize=1&orderBy=timestamp%20desc");
        let auth_token = self.get_auth_token().await?;
        
        let response = self.client
            .get(&url)
            .header(AUTHORIZATION, auth_token)
            .send()
            .await?
            .error_for_status()?;
        
        let result: ListDocumentsResponse = response.json().await?;
        
        if let Some(documents) = result.documents {
            if let Some(doc) = documents.into_iter().next() {
                return Ok(Some(self.firestore_document_to_json(doc)?));
            }
        }
        
        Ok(None)
    }
    
    pub async fn get_price_history(&self, hours: u32) -> Result<Vec<PriceHistory>> {
        let cutoff_time = Tokyo.from_utc_datetime(&chrono::Utc::now().naive_utc()).with_timezone(&FixedOffset::east_opt(9 * 3600).unwrap()) - chrono::Duration::hours(hours as i64);
        let url = format!("{}{}", self.get_collection_url("price_history"), "?orderBy=timestamp%20desc");
        let auth_token = self.get_auth_token().await?;
        
        let response = self.client
            .get(&url)
            .header(AUTHORIZATION, auth_token)
            .send()
            .await?
            .error_for_status()?;
        
        let result: ListDocumentsResponse = response.json().await?;
        let mut prices = Vec::new();
        
        if let Some(documents) = result.documents {
            for doc in documents {
                let price: PriceHistory = self.firestore_document_to_json(doc)?;
                if price.timestamp > cutoff_time {
                    prices.push(price);
                }
            }
        }
        
        Ok(prices)
    }
    
    pub async fn get_price_trend(&self, current_price: Decimal) -> Result<PriceTrend> {
        let now = Tokyo.from_utc_datetime(&chrono::Utc::now().naive_utc()).with_timezone(&FixedOffset::east_opt(9 * 3600).unwrap());
        
        let price_1h = self.get_price_at_time(now - chrono::Duration::hours(1)).await?;
        let price_24h = self.get_price_at_time(now - chrono::Duration::hours(24)).await?;
        let price_7d = self.get_price_at_time(now - chrono::Duration::days(7)).await?;
        
        let trend_1h = price_1h.map(|p| {
            match current_price.cmp(&p) {
                std::cmp::Ordering::Greater => "up".to_string(),
                std::cmp::Ordering::Less => "down".to_string(),
                std::cmp::Ordering::Equal => "stable".to_string(),
            }
        });
        
        let trend_24h = price_24h.map(|p| {
            match current_price.cmp(&p) {
                std::cmp::Ordering::Greater => "up".to_string(),
                std::cmp::Ordering::Less => "down".to_string(),
                std::cmp::Ordering::Equal => "stable".to_string(),
            }
        });
        
        let trend_7d = price_7d.map(|p| {
            match current_price.cmp(&p) {
                std::cmp::Ordering::Greater => "up".to_string(),
                std::cmp::Ordering::Less => "down".to_string(),
                std::cmp::Ordering::Equal => "stable".to_string(),
            }
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
    
    async fn get_price_at_time(&self, time: DateTime<FixedOffset>) -> Result<Option<Decimal>> {
        let url = format!("{}{}", self.get_collection_url("price_history"), "?pageSize=1&orderBy=timestamp%20desc");
        let auth_token = self.get_auth_token().await?;
        
        let response = self.client
            .get(&url)
            .header(AUTHORIZATION, auth_token)
            .send()
            .await?
            .error_for_status()?;
        
        let result: ListDocumentsResponse = response.json().await?;
        
        if let Some(documents) = result.documents {
            for doc in documents {
                let price: PriceHistory = self.firestore_document_to_json(doc)?;
                if price.timestamp <= time {
                    return Ok(Some(price.sol_price_usdc));
                }
            }
        }
        
        Ok(None)
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
        let cutoff_time = Tokyo.from_utc_datetime(&chrono::Utc::now().naive_utc()).with_timezone(&FixedOffset::east_opt(9 * 3600).unwrap()) - chrono::Duration::days(days as i64);
        let url = self.get_collection_url("trading_sessions");
        let auth_token = self.get_auth_token().await?;
        
        let response = self.client
            .get(&url)
            .header(AUTHORIZATION, auth_token)
            .send()
            .await?
            .error_for_status()?;
        
        let result: ListDocumentsResponse = response.json().await?;
        let mut total_trades = 0;
        let mut winning_trades = 0;
        let mut losing_trades = 0;
        let mut total_profit_loss = Decimal::ZERO;
        let mut total_gas_fees = Decimal::ZERO;
        
        if let Some(documents) = result.documents {
            for doc in documents {
                let session: TradingSession = self.firestore_document_to_json(doc)?;
                if session.timestamp > cutoff_time {
                    total_trades += 1;
                    
                    if let Some(profit_loss) = session.profit_loss {
                        total_profit_loss += profit_loss;
                        match profit_loss.cmp(&Decimal::ZERO) {
                            std::cmp::Ordering::Greater => winning_trades += 1,
                            std::cmp::Ordering::Less => losing_trades += 1,
                            std::cmp::Ordering::Equal => {},
                        }
                    }
                    
                    if let Some(gas_fee) = session.gas_fee {
                        total_gas_fees += gas_fee;
                    }
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
        let url = format!("{}{}", self.get_collection_url("profit_tracking"), "?pageSize=1&orderBy=timestamp%20desc");
        let auth_token = self.get_auth_token().await?;
        
        let response = self.client
            .get(&url)
            .header(AUTHORIZATION, auth_token)
            .send()
            .await?
            .error_for_status()?;
        
        let result: ListDocumentsResponse = response.json().await?;
        
        if let Some(documents) = result.documents {
            if let Some(doc) = documents.into_iter().next() {
                return Ok(Some(self.firestore_document_to_json(doc)?));
            }
        }
        
        Ok(None)
    }
    
    pub async fn cleanup_old_data(&self, retention_days: u32) -> Result<()> {
        let cutoff_time = Tokyo.from_utc_datetime(&chrono::Utc::now().naive_utc()).with_timezone(&FixedOffset::east_opt(9 * 3600).unwrap()) - chrono::Duration::days(retention_days as i64);
        
        info!("Cleaning up data older than {} days", retention_days);
        
        // Clean up price history
        let url = self.get_collection_url("price_history");
        let auth_token = self.get_auth_token().await?;
        
        let response = self.client
            .get(&url)
            .header(AUTHORIZATION, &auth_token)
            .send()
            .await?
            .error_for_status()?;
        
        let result: ListDocumentsResponse = response.json().await?;
        let mut deleted_count = 0;
        
        if let Some(documents) = result.documents {
            for doc in documents {
                let price: PriceHistory = self.firestore_document_to_json(doc.clone())?;
                if price.timestamp < cutoff_time {
                    if let Some(name) = doc.name {
                        let delete_url = format!("https://firestore.googleapis.com/v1/{}", name);
                        self.client
                            .delete(&delete_url)
                            .header(AUTHORIZATION, &auth_token)
                            .send()
                            .await?
                            .error_for_status()?;
                        deleted_count += 1;
                    }
                }
            }
        }
        
        info!("Deleted {} old documents from price_history", deleted_count);
        
        // Clean up trading sessions
        let url = self.get_collection_url("trading_sessions");
        let response = self.client
            .get(&url)
            .header(AUTHORIZATION, &auth_token)
            .send()
            .await?
            .error_for_status()?;
        
        let result: ListDocumentsResponse = response.json().await?;
        let mut deleted_count = 0;
        
        if let Some(documents) = result.documents {
            for doc in documents {
                let session: TradingSession = self.firestore_document_to_json(doc.clone())?;
                if session.timestamp < cutoff_time {
                    if let Some(name) = doc.name {
                        let delete_url = format!("https://firestore.googleapis.com/v1/{}", name);
                        self.client
                            .delete(&delete_url)
                            .header(AUTHORIZATION, &auth_token)
                            .send()
                            .await?
                            .error_for_status()?;
                        deleted_count += 1;
                    }
                }
            }
        }
        
        info!("Deleted {} old documents from trading_sessions", deleted_count);
        
        // Clean up profit tracking
        let url = self.get_collection_url("profit_tracking");
        let response = self.client
            .get(&url)
            .header(AUTHORIZATION, &auth_token)
            .send()
            .await?
            .error_for_status()?;
        
        let result: ListDocumentsResponse = response.json().await?;
        let mut deleted_count = 0;
        
        if let Some(documents) = result.documents {
            for doc in documents {
                let profit: ProfitTracking = self.firestore_document_to_json(doc.clone())?;
                if profit.timestamp < cutoff_time {
                    if let Some(name) = doc.name {
                        let delete_url = format!("https://firestore.googleapis.com/v1/{}", name);
                        self.client
                            .delete(&delete_url)
                            .header(AUTHORIZATION, &auth_token)
                            .send()
                            .await?
                            .error_for_status()?;
                        deleted_count += 1;
                    }
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