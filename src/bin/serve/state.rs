
use std::collections::HashMap;
use std::str::FromStr;
use std::hash::Hash;
use std::sync::Arc;


#[derive(Clone)]
pub struct AppState<T> 
    where T: Sync + Send + Clone + 'static
{
    pub providers: Arc<HashMap<Chain, AppProvider<T>>>,
}

pub struct AppProviders<T>(HashMap<Chain, AppProvider<T>>)
    where T: Sync + Send + Clone + 'static;


impl<T> AppProviders<T> 
    where T: Sync + Send + Clone + 'static
{
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn set_provider(
        &mut self,
        chain: Chain,
        endpoint: String,
        handler: Option<T>,
    ) {
        self.0.insert(chain, AppProvider { endpoint, _handler: handler });
    }

    pub fn build(self) -> HashMap<Chain, AppProvider<T>> {
        self.0
    }
}

impl<T> Into<AppState<T>> for AppProviders<T> 
    where T: Sync + Send + Clone + 'static
{
    fn into(self) -> AppState<T> {
        AppState { providers: Arc::new(self.build()) }
    }
}

pub struct AppProvider<T> 
    where T: Sync + Send + Clone + 'static
{
    pub endpoint: String,
    _handler: Option<T>,
}


#[derive(Debug, Eq, Hash, PartialEq)]
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