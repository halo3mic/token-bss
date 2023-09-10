use eyre::Result;

pub struct Config {
    pub eth_rpc_endpoint: String,
}

impl Config {

    pub fn from_env() -> Result<Self> {
        dotenv::dotenv().ok();
        let eth_rpc_endpoint = std::env::var("ETH_RPC")?;
        Ok(Self {
            eth_rpc_endpoint,
        })
    }

}