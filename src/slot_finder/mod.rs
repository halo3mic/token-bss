mod trace_parser;
mod slot_finder;
mod lang;
mod utils;
mod ops;

pub use slot_finder::find_balance_slots_and_update_ratio;
pub use lang::EvmLanguage;
pub mod util {
    pub use super::utils::{env_var, spawn_anvil_provider, spawn_anvil};
} // todo: restrict with feature flag