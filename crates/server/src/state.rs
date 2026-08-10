
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    str::FromStr,
    hash::Hash,
};
use alloy::{
    transports::Transport,
    providers::Provider, 
};
use crate::{
    config::DEFAULT_TIMEOUT_MS,
    db::RedisConnection,
};


#[derive(Clone)]
pub struct AppState<P, T, H> 
    where P: Provider<T>, T: Transport + Clone, H: Sync + Send + Clone + 'static
{
    pub providers: Arc<HashMap<Chain, AppProvider<P, T, H>>>,
    pub db_connection: Option<Arc<Mutex<RedisConnection>>>,
    pub timeout_ms: u64,
}

impl<P, T, H> AppState<P, T, H> 
    where P: Provider<T>, T: Transport + Clone, H: Sync + Send + Clone + 'static
{
    pub fn new(
        providers: HashMap<Chain, AppProvider<P, T, H>>,
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

pub struct AppProviders<P, T, H>(HashMap<Chain, AppProvider<P, T, H>>)
    where P: Provider<T>, T: Transport + Clone, H: Sync + Send + Clone + 'static;

impl<P, T, H> AppProviders<P, T, H> 
    where P: Provider<T>, T: Transport + Clone, H: Sync + Send + Clone + 'static
{
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn set_provider(
        &mut self,
        chain: Chain,
        provider: P,
        local_tracing: bool,
        handler: Option<H>,
    ) {
        self.0.insert(chain, AppProvider { 
            provider: Arc::new(provider),
            _handler: handler,
            local_tracing,
            _phantom_transport: std::marker::PhantomData,
        });
    }

    pub fn build(self) -> HashMap<Chain, AppProvider<P, T, H>> {
        self.0
    }
}

impl<P, T, H> From<AppProviders<P, T, H>> for AppState<P, T, H> 
    where P: Provider<T>, T: Transport + Clone, H: Sync + Send + Clone + 'static
{
    fn from(providers: AppProviders<P, T, H>) -> Self {
        Self {
            providers: Arc::new(providers.build()),
            timeout_ms: DEFAULT_TIMEOUT_MS,
            db_connection: None
        }
    }
}

pub struct AppProvider<P, T, H> 
    where P: Provider<T>, T: Transport + Clone, H: Sync + Send + Clone + 'static
{
    pub provider: Arc<P>,
    pub local_tracing: bool,
    _handler: Option<H>, // Handler for cases like Anvil
    _phantom_transport: std::marker::PhantomData<T>,
}


#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Chain {
    Ethereum, 
    Arbitrum,
    Optimism,
    Avalanche,
    Base,
    Polygon,
    Plasma,
    Unichain,
}

impl Chain {
    /// OP-stack chains need Anvil's `--optimism` flag when forking.
    pub fn is_op_stack(&self) -> bool {
        matches!(self, Chain::Optimism | Chain::Base | Chain::Unichain)
    }
}

impl FromStr for Chain {
    type Err = eyre::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ethereum" | "eth" => Ok(Chain::Ethereum),
            "arbitrum" | "arb" => Ok(Chain::Arbitrum),
            "optimism" | "opt" => Ok(Chain::Optimism),
            "avalanche" | "avax" => Ok(Chain::Avalanche),
            "base" => Ok(Chain::Base),
            "polygon" | "pol" | "matic" => Ok(Chain::Polygon),
            "plasma" => Ok(Chain::Plasma),
            "unichain" | "uni" => Ok(Chain::Unichain),
            _ => Err(eyre::eyre!("Invalid chain")),
        }
    }
}

impl std::fmt::Display for Chain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        format!("{:?}", self).fmt(f)
    }
}