use crate::arbitrage::headers_with_api_key;
use crate::dex::http_utils::HttpRateLimiter;
use crate::dex::quote::{DexClient, Quote};
use crate::utils::{DexType, PoolInfo, PoolParser, PoolToken};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use log::{error, warn};
use reqwest::Client;
// Keep Deserialize for API responses
use serde::Deserialize;
use solana_sdk::pubkey::Pubkey;
use std::env;
use std::str::FromStr;
// For better logging
use std::time::Instant; // For accurate latency measurement

#[derive(Debug, Clone)]
pub struct OrcaClient {
    api_key: String,
    http_client: reqwest::Client,
}

impl OrcaClient {
    pub fn new() -> Self {
        let api_key = env::var("ORCA_API_KEY").unwrap_or_else(|_| {
            warn!("ORCA_API_KEY not set. Depending on the API, this might lead to failures.");
            String::new()
        });
        Self {
            api_key,
            http_client: Client::new(),
        }
    }

    pub fn get_api_key(&self) -> &str {
        &self.api_key
    }
}

// Add a static or global rate limiter for Orca (could be parameterized/configured)
use once_cell::sync::Lazy;
use std::time::Duration;
static ORCA_RATE_LIMITER: Lazy<HttpRateLimiter> = Lazy::new(|| {
    HttpRateLimiter::new(
        4,                          // max concurrent
        Duration::from_millis(200), // min delay between requests
        3,                          // max retries
        Duration::from_millis(250), // base backoff
        vec![
            "".to_string(), // fallback(s) if any
        ],
    )
});

#[async_trait]
impl DexClient for OrcaClient {
    async fn get_best_swap_quote(
        &self,
        input_token: &str,
        output_token: &str,
        amount: u64,
    ) -> Result<Quote> {
        // Use the rate limiter's get_with_backoff to wrap the HTTP request
        let url = format!(
            "https://api.orca.so/v2/solana/quote?inputMint={}&outputMint={}&amountIn={}",
            input_token, output_token, amount
        );
        let request_start_time = Instant::now();
        let response = ORCA_RATE_LIMITER
            .get_with_backoff(&self.http_client, &url, |u| {
                self.http_client
                    .get(u)
                    .headers(headers_with_api_key("ORCA_API_KEY"))
            })
            .await?;
        let request_duration_ms = request_start_time.elapsed().as_millis() as u64;
        if response.status().is_success() {
            let text = response.text().await?;
            // Try to detect if the response is a valid quote or an error
            if text.contains("\"inputMint\"") && text.contains("\"outputMint\"") {
                match serde_json::from_str::<OrcaQuoteResponse>(&text) {
                    Ok(api_response) => {
                        let input_amount_u64 =
                            api_response.in_amount.parse::<u64>().map_err(|e| {
                                anyhow!(
                                    "Failed to parse Orca in_amount '{}': {}",
                                    api_response.in_amount,
                                    e
                                )
                            })?;
                        let output_amount_u64 =
                            api_response.out_amount.parse::<u64>().map_err(|e| {
                                anyhow!(
                                    "Failed to parse Orca out_amount '{}': {}",
                                    api_response.out_amount,
                                    e
                                )
                            })?;
                        Ok(Quote {
                            input_token: api_response.input_mint,
                            output_token: api_response.output_mint,
                            input_amount: input_amount_u64,
                            output_amount: output_amount_u64,
                            dex: self.get_name().to_string(),
                            route: vec![],
                            latency_ms: Some(request_duration_ms),
                            execution_score: None,
                            route_path: None,
                            slippage_estimate: None,
                        })
                    }
                    Err(e) => {
                        error!(
                            "Failed to deserialize Orca quote from URL {}: {:?}. Response body: {}",
                            url, e, text
                        );
                        Err(anyhow!(
                            "Failed to deserialize Orca quote from {}. Error: {}. Body: {}",
                            url,
                            e,
                            text
                        ))
                    }
                }
            } else {
                error!("Orca API did not return a quote. Response body: {}", text);
                Err(anyhow!("Orca API did not return a quote. Body: {}", text))
            }
        } else {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read error response body".to_string());
            error!(
                "Failed to fetch Orca quote from URL {}: Status {}. Body: {}",
                url, status, error_text
            );
            Err(anyhow!(
                "Failed to fetch Orca quote from {}: {}. Body: {}",
                url,
                status,
                error_text
            ))
        }
    }

    fn get_supported_pairs(&self) -> Vec<(String, String)> {
        vec![
            (
                "So11111111111111111111111111111111111111112".to_string(),
                "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
            ), // SOL <-> USDC
            (
                "So11111111111111111111111111111111111111112".to_string(),
                "7vfCXTmUwPXCqU4ZLnXkKvKfvS8qf9oRTXw2qrpJjXWA".to_string(),
            ), // SOL <-> ETH
        ]
    }

    fn get_name(&self) -> &str {
        "OrcaClient"
    }
}

// Define OrcaPoolParser
pub struct OrcaPoolParser;
pub const ORCA_SWAP_PROGRAM_ID: &str = "9W959DqEETiGZoccp2FfeJNjCagVfgtsJy72RykeK2rK";

impl PoolParser for OrcaPoolParser {
    fn parse_pool_data(address: Pubkey, data: &[u8]) -> Result<PoolInfo> {
        if data.len() < 100 {
            error!(
                "Pool parsing failed for {} - Insufficient data length: {}",
                address,
                data.len()
            );
            return Err(anyhow!("Data too short for Orca legacy pool: {}", address));
        }

        Ok(PoolInfo {
            address,
            name: format!(
                "OrcaPool/{}",
                address.to_string().chars().take(6).collect::<String>()
            ),
            token_a: PoolToken {
                mint: Pubkey::new_unique(),
                symbol: "TKA".to_string(),
                decimals: 6,
                reserve: 1_000_000,
            },
            token_b: PoolToken {
                mint: Pubkey::new_unique(),
                symbol: "TKB".to_string(),
                decimals: 6,
                reserve: 1_000_000,
            },
            fee_numerator: 30,
            fee_denominator: 10000,
            last_update_timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            dex_type: DexType::Orca,
        })
    }

    fn get_program_id() -> Pubkey {
        Pubkey::from_str(ORCA_SWAP_PROGRAM_ID).unwrap()
    }

    fn get_dex_type() -> DexType {
        DexType::Orca
    }
}

impl OrcaPoolParser {
    pub fn parse_pool_data(address: Pubkey, data: &[u8]) -> anyhow::Result<PoolInfo> {
        <Self as PoolParser>::parse_pool_data(address, data)
    }
    pub fn get_program_id() -> Pubkey {
        <Self as PoolParser>::get_program_id()
    }
}

// This struct matches the expected fields in a successful Orca quote API response.
// If the API returns an error or a different structure, deserialization will fail.
#[derive(Deserialize, Debug)]
struct OrcaQuoteResponse {
    #[serde(rename = "inputMint")]
    input_mint: String,
    #[serde(rename = "outputMint")]
    output_mint: String,
    #[serde(rename = "inAmount")]
    in_amount: String,
    #[serde(rename = "outAmount")]
    out_amount: String,
    // ...other fields as needed...
}
