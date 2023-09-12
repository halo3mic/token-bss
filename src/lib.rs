pub mod slot_finder;
pub mod config;
pub mod utils;

use eyre::Result;
use ethers::{
    providers::{Http, Provider},
    types::{H160, U256},
};


pub async fn find_slot(provider_url: &str, token: H160) -> Result<(H160, U256, f64)> {
    let provider = Provider::<Http>::try_from(provider_url)?;
    let holder = H160::from_low_u64_be(1);
    let (contract, slot, update_ratio) = 
        slot_finder::find_balance_slots_and_update_ratio(&provider, holder, token).await?;
    
    Ok((contract, slot, update_ratio))
}

