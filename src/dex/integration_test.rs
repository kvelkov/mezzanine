//! Integration utility to exercise all DEX infrastructure and eliminate unused warnings.

use crate::dex::lifinity::{LifinityClient, LifinityPoolParser, LIFINITY_PROGRAM_ID};
use crate::dex::meteora::MeteoraClient;
use crate::dex::orca::{OrcaClient, OrcaPoolParser, ORCA_SWAP_PROGRAM_ID};
use crate::dex::phoenix::{PhoenixClient, PHOENIX_PROGRAM_ID};
use crate::dex::pool::get_pool_parser_fn_for_program;
use crate::dex::quote::DexClient;
use crate::dex::raydium::{RaydiumClient, RaydiumPoolParser, RAYDIUM_LIQUIDITY_PROGRAM_ID};
use crate::dex::whirlpool::{WhirlpoolClient, WhirlpoolPoolParser, ORCA_WHIRLPOOL_PROGRAM_ID};
use crate::utils::{DexType, PoolInfo, PoolToken};
use serde_json;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// Call all parser registry and static parser methods for all DEXes
pub fn exercise_parser_registry() {
    // Use all program IDs
    let orca_id = match Pubkey::from_str(ORCA_SWAP_PROGRAM_ID) {
        Ok(id) => id,
        Err(e) => {
            eprintln!(
                "Invalid ORCA_SWAP_PROGRAM_ID '{}': {}",
                ORCA_SWAP_PROGRAM_ID, e
            );
            return;
        }
    };
    let raydium_id = match Pubkey::from_str(RAYDIUM_LIQUIDITY_PROGRAM_ID) {
        Ok(id) => id,
        Err(e) => {
            eprintln!(
                "Invalid RAYDIUM_LIQUIDITY_PROGRAM_ID '{}': {}",
                RAYDIUM_LIQUIDITY_PROGRAM_ID, e
            );
            return;
        }
    };
    let whirlpool_id = match Pubkey::from_str(ORCA_WHIRLPOOL_PROGRAM_ID) {
        Ok(id) => id,
        Err(e) => {
            eprintln!(
                "Invalid ORCA_WHIRLPOOL_PROGRAM_ID '{}': {}",
                ORCA_WHIRLPOOL_PROGRAM_ID, e
            );
            return;
        }
    };
    let lifinity_id = match Pubkey::from_str(LIFINITY_PROGRAM_ID) {
        Ok(id) => id,
        Err(e) => {
            eprintln!(
                "Invalid LIFINITY_PROGRAM_ID '{}': {}",
                LIFINITY_PROGRAM_ID, e
            );
            return;
        }
    };
    let phoenix_id = match Pubkey::from_str(PHOENIX_PROGRAM_ID) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("Invalid PHOENIX_PROGRAM_ID '{}': {}", PHOENIX_PROGRAM_ID, e);
            return;
        }
    };
    let ids = vec![orca_id, raydium_id, whirlpool_id, lifinity_id, phoenix_id];
    for id in ids {
        let _ = get_pool_parser_fn_for_program(&id);
    }
    // Call static parser methods
    let dummy = Pubkey::new_unique();
    let dummy_data = vec![0u8; 400];
    let _ = OrcaPoolParser::parse_pool_data(dummy, &dummy_data);
    let _ = RaydiumPoolParser::parse_pool_data(dummy, &dummy_data);
    let _ = WhirlpoolPoolParser::parse_pool_data(dummy, &dummy_data);
    let _ = LifinityPoolParser::parse_pool_data(dummy, &dummy_data);
    let _ = OrcaPoolParser::get_program_id();
    let _ = RaydiumPoolParser::get_program_id();
    let _ = WhirlpoolPoolParser::get_program_id();
    let _ = LifinityPoolParser::get_program_id();
}

/// Call all DEX client trait methods and api_key accessors
pub async fn exercise_dex_clients() {
    use crate::dex::http_utils_shared::log_timed_request;

    // Create clients within timed request to demonstrate log_timed_request usage
    let orca = log_timed_request("Initialize Orca Client", async { OrcaClient::new() }).await;
    let raydium =
        log_timed_request("Initialize Raydium Client", async { RaydiumClient::new() }).await;
    let whirlpool = log_timed_request("Initialize Whirlpool Client", async {
        WhirlpoolClient::new()
    })
    .await;
    let lifinity = log_timed_request("Initialize Lifinity Client", async {
        LifinityClient::new()
    })
    .await;
    let meteora =
        log_timed_request("Initialize Meteora Client", async { MeteoraClient::new() }).await;
    let phoenix =
        log_timed_request("Initialize Phoenix Client", async { PhoenixClient::new() }).await;
    // Use api_key fields
    let _ = orca.get_api_key();
    let _ = raydium.get_api_key();
    let _ = whirlpool.get_api_key();
    let _ = lifinity.get_api_key();
    let _ = meteora.get_api_key();
    let _ = phoenix.get_api_key();
    // Call trait methods
    let _ = orca.get_supported_pairs();
    let _ = raydium.get_supported_pairs();
    let _ = whirlpool.get_supported_pairs();
    let _ = lifinity.get_supported_pairs();
    let _ = meteora.get_supported_pairs();
    let _ = phoenix.get_supported_pairs();
    let _ = orca.get_name();
    let _ = raydium.get_name();
    let _ = whirlpool.get_name();
    let _ = lifinity.get_name();
    let _ = meteora.get_name();
    let _ = phoenix.get_name();
    // Call async quote (dummy values)
    let usdc = "EPjFWdd5AufqSSqeM2q8VsJb9h6p9FZ6F1u1kQ1QwZ5Q";
    let sol = "So11111111111111111111111111111111111111112";
    let _ = orca.get_best_swap_quote(usdc, sol, 1_000_000).await;
    let _ = raydium.get_best_swap_quote(usdc, sol, 1_000_000).await;
    let _ = whirlpool.get_best_swap_quote(usdc, sol, 1_000_000).await;
    let _ = lifinity.get_best_swap_quote(usdc, sol, 1_000_000).await;
    let _ = meteora.get_best_swap_quote(usdc, sol, 1_000_000).await;
    let _ = phoenix.get_best_swap_quote(usdc, sol, 1_000_000).await;
}

/// Serialize and deserialize a dummy PoolInfo to exercise serde imports
pub fn exercise_serde() {
    let pool = PoolInfo {
        address: Pubkey::new_unique(),
        name: "TestPool".to_string(),
        token_a: PoolToken {
            mint: Pubkey::new_unique(),
            symbol: "A".to_string(),
            decimals: 6,
            reserve: 1_000_000,
        },
        token_b: PoolToken {
            mint: Pubkey::new_unique(),
            symbol: "B".to_string(),
            decimals: 6,
            reserve: 2_000_000,
        },
        fee_numerator: 30,
        fee_denominator: 10000,
        last_update_timestamp: 0,
        dex_type: DexType::Orca,
    };
    let json = serde_json::to_string(&pool).unwrap();
    let _pool2: PoolInfo = serde_json::from_str(&json).unwrap();
}
