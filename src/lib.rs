pub mod slot_finder;
pub mod config;
pub mod utils;

use eyre::Result;
use ethers::{
    providers::{Http, Provider},
    types::{H160, U256, H256},
};


pub async fn find_slot(
    provider_url: String, 
    token: H160, 
    holder: Option<H160>
) -> Result<(H160, H256, f64, String)> {
    let provider = Provider::<Http>::try_from(&provider_url)?;
    let holder = holder.unwrap_or(H160::from_low_u64_be(1));
    let (contract, slot, update_ratio, lang) = 
        slot_finder::find_balance_slots_and_update_ratio(
            &provider, 
            holder, 
            token
        ).await?;
    
    Ok((contract, slot, update_ratio, lang))
}

// ? use update_ratio to set a target balance (if fee is 2% take it into account)
pub async fn set_balance(
    provider_url: String, 
    token: H160, 
    holder: H160,
    target_balance: f64, 
    slot_info: Option<(H160, H256, f64, String)>
) -> Result<U256> {
    let provider = Provider::<Http>::try_from(&provider_url)?;
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
