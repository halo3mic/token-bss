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
use alloy::transports::http::Http;
use alloy::network::Ethereum;
use reqwest::Client;
use std::sync::Arc;
use state::{Chain, AppProviders};
use config::RpcUrl;


fn make_app_http_providers(configs: &Config) -> Result<AppProviders<Http<Client>, Arc<AnvilInstance>>> {
    let mut app_providers = state::AppProviders::new();
    for chain_config in &configs.chain_configs {
        let (provider, handler) = match &chain_config.rpc_url {
            RpcUrl::Primary(url) => {
                info!("Added ReqwestProvider of Anvil fork for chain: {:?}", chain_config.chain);
                (ReqwestProvider::<Ethereum>::new_http(url.parse().unwrap()).into(), None)
            }
            RpcUrl::Fork(url) => {
                if let Some(anvil_config) = &configs.anvil_config {
                    let anvil = utils::spawn_anvil(
                        Some(&url),
                        Some(anvil_config),
                        Some(matches!(chain_config.chain, Chain::Optimism)),
                    );
                    let provider = ReqwestProvider::<Ethereum>::new_http(anvil.endpoint_url());
                    info!("Added ReqwestProvider of Anvil fork for chain: {:?}", chain_config.chain);
                    (provider.into(), Some(Arc::new(anvil)))
                } else {
                    let provider = ReqwestProvider::<Ethereum>::new_http(url.parse()?);
                    let provider = poor_mans_tracer::LocalTraceProvider::new(provider);
                    info!("Added LocalTraceProvider for chain: {:?}", chain_config.chain);
                    (provider.into(), None)
                }
            },
        };
        app_providers.set_provider(chain_config.chain, provider, handler);
    }

    Ok(app_providers)
}
