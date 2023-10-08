/// Check balanceOf slot detection for popular tokens on Ethereum
use ethers::prelude::*;
use eyre::Result;
use serde::Deserialize;


#[derive(Deserialize)]
struct CoingeckoApiResp {
    tokens: Vec<TokenInfo>,
}

// #[serde(rename_all = "camelCase")]
#[derive(Deserialize)]
struct TokenInfo {
    // chain_id: u64,
    address: H160,
    // name: String,
    symbol: String,
    // decimals: u8,
    // #[serde(rename = "logoURI")]
    // logo_uri: Option<String>,
}

async fn coingecko_all_tokens(network_id: String) -> Result<Vec<(String, H160)>> {
    let url = format!("https://tokens.coingecko.com/{network_id}/all.json");
    let response = reqwest::get(&url).await?;
    let api_resp: CoingeckoApiResp = response.json().await?;
    api_resp.tokens.into_iter().map(|t| Ok((t.symbol, t.address))).collect()
}

#[tokio::test]
async fn test_popular_tokens_support() -> Result<()> {
    let ethereum_tokens = coingecko_all_tokens("ethereum".to_string()).await?;
    let config = erc20_topup::config::Config::from_env()?;
    let anvil = erc20_topup::utils::spawn_anvil(Some(&config.eth_rpc_endpoint));
    
    for (symbol, token) in ethereum_tokens {
        println!("Checking {symbol}({token:?})");
        match erc20_topup::find_slot(anvil.endpoint(), token, None).await {
            Ok((contract, slot, update_ratio)) => println!("{symbol}({token:?}): {contract:?} - {slot:?} / ΔR: {update_ratio}"),
            Err(e) => println!("{symbol}({token:?}): {e}"),
        }
    }

    Ok(())
}