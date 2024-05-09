
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    str::FromStr,
    hash::Hash,
};
use alloy::{
    providers::RootProvider, 
    transports::Transport, 
    network::Ethereum,
};
use crate::{
    config::DEFAULT_TIMEOUT_MS,
    db::RedisConnection,
};
use poor_mans_tracer::LocalTraceProvider;


#[derive(Clone)]
pub enum ProviderType<T> 
    where T: Transport + Clone
{
    RootProvider(RootProvider<T>),
    LocalTraceProvider(LocalTraceProvider<T, Ethereum>),
}

impl<T> From<RootProvider<T>> for ProviderType<T> 
    where T: Transport + Clone
{
    fn from(provider: RootProvider<T, Ethereum>) -> Self {
        Self::RootProvider(provider)
    }
}

impl<T> From<LocalTraceProvider<T, Ethereum>> for ProviderType<T> 
    where T: Transport + Clone
{
    fn from(provider: LocalTraceProvider<T, Ethereum>) -> Self {
        Self::LocalTraceProvider(provider)
    }
}

#[derive(Clone)]
pub struct AppState<T, H> 
    where T: Transport + Clone, H: Sync + Send + Clone + 'static
{
    pub providers: Arc<HashMap<Chain, AppProvider<T, H>>>,
    pub db_connection: Option<Arc<Mutex<RedisConnection>>>,
    pub timeout_ms: u64,
}

impl<T, H> AppState<T, H> 
    where T: Transport + Clone, H: Sync + Send + Clone + 'static
{

    pub fn new(
        providers: HashMap<Chain, AppProvider<T, H>>,
        db_connection: Option<RedisConnection>,
        timeout_ms: u64,
    ) -> Self {
        Self {
            providers: Arc::new(providers),
            db_connection: db_connection.map(|conn| Arc::new(Mutex::new(conn))),
            timeout_ms,
        }
    }
}

pub struct AppProviders<T, H>(HashMap<Chain, AppProvider<T, H>>)
    where T: Transport + Clone, H: Sync + Send + Clone + 'static;

impl<T, H> AppProviders<T, H> 
    where T: Transport + Clone, H: Sync + Send + Clone + 'static
{
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn set_provider(
        &mut self,
        chain: Chain,
        provider: ProviderType<T>,
        handler: Option<H>,
    ) {
        self.0.insert(chain, AppProvider { 
            provider, 
            _handler: handler, 
        });
    }

    pub fn build(self) -> HashMap<Chain, AppProvider<T, H>> {
        self.0
    }
}

impl<T, H> From<AppProviders<T, H>> for AppState<T, H> 
    where T: Transport + Clone, H: Sync + Send + Clone + 'static
{
    fn from(providers: AppProviders<T, H>) -> Self {
        Self {
            providers: Arc::new(providers.build()),
            timeout_ms: DEFAULT_TIMEOUT_MS,
            db_connection: None
        }
    }
}

pub struct AppProvider<T, H> 
    where T: Transport + Clone, H: Sync + Send + Clone + 'static
{
    pub provider: ProviderType<T>,
    _handler: Option<H>, // Handler for cases like Anvil
}


#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Chain {
    Ethereum, 
    Arbitrum,
    Optimism,
    Avalanche,
}

impl FromStr for Chain {
    type Err = eyre::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ethereum" | "eth" => Ok(Chain::Ethereum),
            "arbitrum" | "arb" => Ok(Chain::Arbitrum),
            "optimism" | "opt" => Ok(Chain::Optimism),
            "avalanche" | "avax" => Ok(Chain::Avalanche),
            _ => Err(eyre::eyre!("Invalid chain")),
        }
    }
}

impl std::fmt::Display for Chain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        format!("{:?}", self).fmt(f)
    }
}