mod handlers;
mod server;
mod utils;
mod state;
mod config;
mod db;

use alloy::providers::ReqwestProvider;
use alloy::network::Ethereum;
use std::sync::Arc;
use tracing::info;
use eyre::Result;
use config::{Config, RpcUrl};


#[tokio::main]
async fn main() -> Result<()> {
    let configs = Config::from_env()?;

    if configs.chain_configs.is_empty() {
        return Err(eyre::eyre!("No chain configs found"));
    }
    if configs.logging_enabled {
        tracing_subscriber::fmt::init();
    }

    let mut app_providers = state::AppProviders::new();
    // todo: move this somewhere else
    for chain_config in configs.chain_configs {
        let (endpoint, handler) = match chain_config.rpc_url {
            RpcUrl::Primary(url) => (url, None),
            RpcUrl::Fork(url) => {
                let anvil = utils::spawn_anvil(
                    Some(&url), 
                    Some(&configs.anvil_config),
                    Some(matches!(chain_config.chain, state::Chain::Optimism)),
                );
                (anvil.endpoint(), Some(Arc::new(anvil)))
            },
        };
        info!("Added provider for chain: {:?} with endpoint {endpoint:?}", chain_config.chain);
        let provider = ReqwestProvider::<Ethereum>::new_http(endpoint.parse()?); // todo: move this somewhere else
        app_providers.set_provider(chain_config.chain, provider, handler);
    }

    let redis_conn = configs.redis_config.map(|c| c.try_into()).transpose()?;
    let app_state = state::AppState::new(
        app_providers.build(), 
        redis_conn,
        configs.timeout_ms,
    );

    server::run(&configs.server_addr, app_state).await?;

    Ok(())

}
