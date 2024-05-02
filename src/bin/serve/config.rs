use super::state::Chain;
use eyre::Result;


pub const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: u32 = 3000;

pub struct Config {
    pub server_addr: String,
    pub chain_configs: Vec<ChainConfig>,
}

pub struct ChainConfig {
    pub chain: Chain,
    pub rpc_url: RpcUrl,
}

#[derive(Debug)]
pub enum RpcUrl {
    Primary(String),
    Fallack(String),
}

impl Config {

    pub fn from_env() -> Result<Self> {
        let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("./src/bin/serve/.env");
        dotenv::from_path(path).ok();

        let host = std::env::var("SERVER_HOST").ok()
            .and_then(|s| (!s.trim().is_empty()).then(|| s))
            .unwrap_or(DEFAULT_HOST.to_string());
        let port = std::env::var("SERVER_PORT").ok()
            .and_then(|p_str| p_str.parse::<u32>().ok())
            .unwrap_or(DEFAULT_PORT);
        let server_addr = format!("{}:{}", host, port);

        let available_chains = vec![
            Chain::Ethereum,
            Chain::Arbitrum,
            Chain::Optimism,
            Chain::Avalanche,
        ];
        let chain_configs = available_chains.into_iter().filter_map(|chain| {
            let primary_key = format!("{}_RPC", chain.to_string().to_uppercase());
            std::env::var(primary_key).ok()
                .and_then(|s| (!s.trim().is_empty()).then(|| s))
                .map(RpcUrl::Primary)
                .or_else(|| {
                    let fallback_key = format!("{}_FORK_RPC", chain.to_string().to_uppercase());
                    std::env::var(fallback_key).ok()
                        .and_then(|s| (!s.trim().is_empty()).then(|| s))
                        .map(RpcUrl::Fallack)
                })
                .map(|url| ChainConfig { chain, rpc_url: url })
        }).collect::<Vec<_>>();

        Ok(Self {
            chain_configs,
            server_addr,
        })
    }
}