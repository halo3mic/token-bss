use alloy::primitives::Address;
use eyre::Result;
use serde::Deserialize;


#[derive(Deserialize)]
struct CoingeckoApiResp {
    tokens: Vec<TokenInfo>,
}

#[derive(Deserialize)]
struct TokenInfo {
    address: Address,
    symbol: String,
}

async fn coingecko_all_tokens(network_id: String) -> Result<Vec<(String, Address)>> {
    let url = format!("https://tokens.coingecko.com/{network_id}/all.json");
    let response = reqwest::get(&url).await?;
    let api_resp: CoingeckoApiResp = response.json().await?;
    api_resp.tokens.into_iter().map(|t| Ok((t.symbol, t.address))).collect()
}

#[tokio::test]
async fn test_popular_tokens_support() -> Result<()> {
    let ethereum_tokens = coingecko_all_tokens("ethereum".to_string()).await?;
    let rpc_endpoint = erc20_topup::utils::env_var("RPC_URL")?;
    let anvil = erc20_topup::utils::spawn_anvil(Some(&rpc_endpoint));
    
    for (symbol, token) in ethereum_tokens {
        println!("Checking {symbol}({token:?})");
        match erc20_topup::find_slot(&anvil.endpoint(), token, None).await {
            Ok((contract, slot, update_ratio, lang)) => {
                println!("{symbol}({token:?}): {contract:?}({lang}) - {slot:?} / ΔR: {update_ratio}")
            }
            Err(e) => println!("{symbol}({token:?}): {e}"),
        }
    }

    Ok(())
}