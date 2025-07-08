use anyhow::Result;
use std::future::Future;
use std::time::Duration;
use tokio::time::{sleep, timeout};
use tracing::{debug, error, warn};

pub async fn retry_as_exponential_back_off<F, Fut, T, E>(
    mut operation: F,
    operation_name: &str,
    max_retries: u32,
    initial_delay_ms: u64,
    timeout_duration: Option<Duration>,
) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::fmt::Display + Send + Sync + 'static,
{
    let mut retry_delay = Duration::from_millis(initial_delay_ms);
    
    for attempt in 0..max_retries {
        let result = if let Some(timeout_dur) = timeout_duration {
            match timeout(timeout_dur, operation()).await {
                Ok(result) => result,
                Err(_) => {
                    let error_msg = format!("{} timeout", operation_name);
                    if attempt < max_retries - 1 {
                        warn!(
                            "{} (attempt {}/{}). Retrying in {:?}...",
                            error_msg,
                            attempt + 1,
                            max_retries,
                            retry_delay
                        );
                        sleep(retry_delay).await;
                        retry_delay *= 2;
                        continue;
                    } else {
                        error!("{} after {} attempts", error_msg, max_retries);
                        return Err(anyhow::anyhow!(
                            "{} after {} attempts",
                            error_msg,
                            max_retries
                        ));
                    }
                }
            }
        } else {
            operation().await
        };
        
        match result {
            Ok(value) => {
                if attempt > 0 {
                    debug!("Successfully completed {} on attempt {}", operation_name, attempt + 1);
                }
                return Ok(value);
            }
            Err(e) => {
                let error_msg = format!("{} error: {}", operation_name, e);
                
                if attempt < max_retries - 1 {
                    warn!(
                        "{} (attempt {}/{}). Retrying in {:?}...",
                        error_msg,
                        attempt + 1,
                        max_retries,
                        retry_delay
                    );
                    sleep(retry_delay).await;
                    retry_delay *= 2;
                } else {
                    error!("{} failed after {} attempts: {}", operation_name, max_retries, e);
                    return Err(anyhow::anyhow!(
                        "{} failed after {} attempts: {}",
                        operation_name,
                        max_retries,
                        e
                    ));
                }
            }
        }
    }
    
    unreachable!("Should have returned from the retry loop")
}