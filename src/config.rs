use eyre::Result;

pub struct Config {
    pub rpc_endpoint: String,
}

impl Config {

    pub fn from_env() -> Result<Self> {
        dotenv::dotenv().ok();
        let rpc_endpoint = std::env::var("RPC_URL")?;
        Ok(Self {
            rpc_endpoint,
        })
    }

}