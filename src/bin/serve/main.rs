mod handlers;
mod server;
mod utils;
mod state;
mod config;

use std::sync::Arc;
use tracing::info;
use eyre::Result;
use config::{Config, RpcUrl};


#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::from_env()?;
    if config.chain_configs.is_empty() {
        return Err(eyre::eyre!("No chain configs found"));
    }

    tracing_subscriber::fmt::init(); // todo: only if logging is on

    let mut app_state = state::AppProviders::new();
    for chain_config in config.chain_configs {
        let (endpoint, handler) = match chain_config.rpc_url {
            RpcUrl::Primary(url) => (url, None),
            RpcUrl::Fallack(url) => {
                let anvil = utils::spawn_anvil(Some(&url));
                (anvil.endpoint(), Some(Arc::new(anvil)))
            },
        };
        info!("Added provider for chain: {:?} with endpoint {endpoint:?}", chain_config.chain);
        app_state.set_provider(chain_config.chain, endpoint, handler);
    }

    server::run(&config.server_addr, app_state.into()).await?;

    Ok(())

}
