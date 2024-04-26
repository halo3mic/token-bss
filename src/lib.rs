pub mod slot_finder;
pub mod utils;

use eyre::Result;
use alloy::{
    primitives::{Address, B256, U256}, 
    providers::RootProvider,
    transports::http::Http,
};
use reqwest::Client;
// todo: RootProvider<Http<Client>> could go into common or smth


pub async fn find_slot(
    provider_url: &str, 
    token: Address, 
    holder: Option<Address>
) -> Result<(Address, B256, f64, String)> {
    let provider = http_provider_from_url(provider_url);
    let holder = holder.unwrap_or(Address::from_word(B256::from(U256::from(0))));
    let (contract, slot, update_ratio, lang) = 
        slot_finder::find_balance_slots_and_update_ratio(
            &provider, 
            holder, 
            token
        ).await?;
    
    Ok((contract, slot, update_ratio, lang))
}

// todo: after using eth_call with overrides it doesn't make sens to have this here -> move to utils
// ? use update_ratio to set a target balance (if fee is 2% take it into account)
pub async fn set_balance(
    provider_url: &str, 
    token: Address, 
    holder: Address,
    target_balance: f64, 
    slot_info: Option<(Address, B256, f64, String)>
) -> Result<U256> {
    let provider = http_provider_from_url(provider_url);
    let (contract, slot, _update_ratio, lang) = match slot_info {
        Some(slot_info) => slot_info,
        None => {
            slot_finder::find_balance_slots_and_update_ratio(
                &provider, 
                holder, 
                token
            ).await?
        }
    };
    let target_bal_fixed = utils::token_dec_to_fixed(&provider, token, target_balance).await?;
    let resulting_balance = slot_finder::update_balance(
        &provider, 
        token, 
        holder,
        target_bal_fixed,
        contract, 
        slot, 
        lang,
    ).await?;

    Ok(resulting_balance)
}


fn http_provider_from_url(url: &str) -> RootProvider<Http<Client>> {
    RootProvider::<Http<Client>>::new_http(url.parse().unwrap())
}