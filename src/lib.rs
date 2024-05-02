#![allow(clippy::module_inception)]

mod common;
mod slot_finder;

pub use slot_finder::EvmLanguage;
use common::*;

pub async fn find_slot(
    provider_url: &str, 
    token: Address, 
    holder: Option<Address>
) -> Result<(Address, B256, f64, String)> {
    let provider = http_provider_from_url(provider_url);
    let holder = holder.unwrap_or_else(default_holder);

    slot_finder::find_balance_slots_and_update_ratio(
        &provider, 
        holder, 
        token
    ).await
}

// todo: instead of url accept Provider so IPC can be used
fn http_provider_from_url(url: &str) -> RootProviderHttp {
    RootProviderHttp::new_http(url.parse().unwrap())
}

// Avoid zero address for holder
fn default_holder() -> Address {
    Address::from_word(B256::from(U256::from(1)))
}