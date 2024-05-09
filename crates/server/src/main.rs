mod handlers;
mod config;
mod server;
mod utils;
mod state;
mod db;

use tracing::info;
use eyre::Result;
use config::Config;


#[tokio::main]
async fn main() -> Result<()> {
    let configs = Config::from_env()?;
    if configs.chain_configs.is_empty() {
        return Err(eyre::eyre!("No chain configs found"));
    }
    if configs.logging_enabled {
        tracing_subscriber::fmt::init();
    }
    let app_providers = make_app_http_providers(&configs)?;
    let redis_conn = configs.redis_config.map(|c| c.try_into()).transpose()?;
    let app_state = state::AppState::new(
        app_providers.build(), 
        redis_conn,
        configs.timeout_ms,
    );
    server::run(&configs.server_addr, app_state).await?;
    Ok(())
}


use alloy::node_bindings::AnvilInstance;
use alloy::providers::ReqwestProvider;
use alloy::network::Ethereum;
use alloy::transports::http::Http;
use reqwest::Client;
use std::sync::Arc;
use state::{Chain, AppProviders};
use config::RpcUrl;


fn make_app_http_providers(configs: &Config) -> Result<AppProviders<ReqwestProvider<Ethereum>, Http<Client>, Arc<AnvilInstance>>> {
    let mut app_providers = state::AppProviders::new();
    for chain_config in &configs.chain_configs {
        let (endpoint, handler, local_tracing) = match &chain_config.rpc_url {
            RpcUrl::Primary(url) => {
                info!("Added provider for chain: {:?}", chain_config.chain);
                (url.clone(), None, false)
            }
            RpcUrl::Fork(url) => {
                if let Some(anvil_config) = &configs.anvil_config {
                    let anvil = utils::spawn_anvil(
                        Some(&url),
                        Some(anvil_config),
                        Some(matches!(chain_config.chain, Chain::Optimism)),
                    );
                    let anvil_endpoint = anvil.endpoint(); // Store the value of anvil.endpoint() in a variable
                    info!("Added provider of Anvil fork for chain: {:?}", chain_config.chain);
                    (anvil_endpoint, Some(Arc::new(anvil)), false) // Use the variable instead of calling anvil.endpoint() directly
                } else {
                    info!("Added provider with local tracing for chain: {:?}", chain_config.chain);
                    (url.clone(), None, true)
                }
            },
        };
        let provider = ReqwestProvider::<Ethereum>::new_http(endpoint.parse()?);
        app_providers.set_provider(chain_config.chain, provider, local_tracing, handler);
    }

    Ok(app_providers)
}
