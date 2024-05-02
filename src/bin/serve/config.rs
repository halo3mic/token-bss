use super::state::Chain;
use eyre::Result;


pub const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: u32 = 3000;

pub struct Config {
    pub server_addr: String,
    pub chain_configs: Vec<ChainConfig>,
    pub redis_config: Option<RedisConfig>,
    pub logging_enabled: bool,
}

pub struct ChainConfig {
    pub chain: Chain,
    pub rpc_url: RpcUrl,
}

pub enum RpcUrl {
    Primary(String),
    Fallack(String),
}

#[derive(Default)]
pub struct RedisConfig {
    pub addr: String,
    pub password: Option<String>,
    pub is_tls: bool,
}

impl Config {

    pub fn from_env() -> Result<Self> {
        let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("./src/bin/serve/.env"); // todo: make this more robust
        dotenv::from_path(path).ok();

        // Server config
        let host = std::env::var("SERVER_HOST").ok()
            .and_then(|s| (!s.trim().is_empty()).then(|| s))
            .unwrap_or(DEFAULT_HOST.to_string());
        let port = std::env::var("SERVER_PORT").ok()
            .and_then(|p_str| p_str.parse::<u32>().ok())
            .unwrap_or(DEFAULT_PORT);
        let server_addr = format!("{}:{}", host, port);

        // Chain & RPCs config
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

        // Redis config
        let redis_enabled = std::env::var("REDIS_ENABLED").ok()
            .map(|s| s == "1").unwrap_or(false);
        let redis_config = 
            if redis_enabled {
                let mut redis_config = RedisConfig::default();
                let redis_host = std::env::var("REDIS_HOST").ok()
                    .and_then(|s| (!s.trim().is_empty()).then(|| s))
                    .unwrap_or("localhost".to_string());
                let redis_port = std::env::var("REDIS_PORT").ok()
                    .and_then(|p_str| p_str.parse::<u32>().ok())
                    .unwrap_or(6379);
                redis_config.addr = format!("{}:{}", redis_host, redis_port);
                redis_config.password = std::env::var("REDIS_PASSWORD").ok();
                redis_config.is_tls = std::env::var("REDIS_IS_TLS").ok()
                    .map(|s| s == "1").unwrap_or(false);
                Some(redis_config)
            } else {
                None
            };
        
        // Logging config
        let logging_enabled = std::env::var("LOGGING_ENABLED").ok()
            .map(|s| s == "1").unwrap_or(false);

        Ok(Self {
            logging_enabled,
            chain_configs,
            redis_config,
            server_addr,
        })
    }
}