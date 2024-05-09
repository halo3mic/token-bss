mod trace_parser;
mod slot_finder;
mod lang;
mod utils;
mod ops;


pub use slot_finder::find_balance_slots_and_update_ratio;
pub use lang::EvmLanguage;

mod common;

pub use common::TraceFn;
use common::*;

pub async fn find_slot<P, T>(
    provider: &P, 
    token: Address, 
    holder: Option<Address>,
    trace_fn: Option<TraceFn>,
) -> Result<(Address, B256, f64, String)> 
    where P: Provider<T>, T: Transport + Clone
{
    let holder = holder.unwrap_or_else(default_holder);
    slot_finder::find_balance_slots_and_update_ratio(
        provider, 
        holder, 
        token,
        trace_fn,
    ).await
}

// Avoid zero address for holder
fn default_holder() -> Address {
    Address::from_word(B256::from(U256::from(1)))
}