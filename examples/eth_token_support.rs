use alloy::node_bindings::{Anvil, AnvilInstance};
use alloy::providers::ReqwestProvider;
use alloy::primitives::Address;
use serde::Deserialize;
use eyre::Result;


#[derive(Deserialize)]
struct CoingeckoApiResp {
    tokens: Vec<TokenInfo>,
}

#[derive(Deserialize)]
struct TokenInfo {
    address: Address,
    symbol: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let ethereum_tokens = coingecko_all_tokens("ethereum".to_string()).await?;
    let rpc_endpoint = env_var("ETH_RPC_URL")?;
    let anvil = spawn_anvil(Some(&rpc_endpoint));
    let provider = ReqwestProvider::new_http(anvil.endpoint().parse()?);
    
    for (symbol, token) in ethereum_tokens {
        println!("Checking {symbol}({token:?})");
        match token_bss::find_slot(&provider, token, None, None).await {
            Ok((contract, slot, update_ratio, lang)) => {
                println!("{symbol}({token:?}): {contract:?}({lang}) - {slot:?} / ΔR: {update_ratio}")
            }
            Err(e) => println!("{symbol}({token:?}): {e}"),
        }
    }

    Ok(())
}

async fn coingecko_all_tokens(network_id: String) -> Result<Vec<(String, Address)>> {
    let url = format!("https://tokens.coingecko.com/{network_id}/all.json");
    let response = reqwest::get(&url).await?;
    let api_resp: CoingeckoApiResp = response.json().await?;
    api_resp.tokens.into_iter().map(|t| Ok((t.symbol, t.address))).collect()
}

fn env_var(var: &str) -> Result<String> {
    dotenv::dotenv().ok();
    std::env::var(var).map_err(|_| eyre::eyre!("{} not set", var))
}

fn spawn_anvil(fork_url: Option<&str>) -> AnvilInstance {
    (match fork_url {
        Some(url) => Anvil::new().fork(url),
        None => Anvil::new(),
    }).spawn()
} 