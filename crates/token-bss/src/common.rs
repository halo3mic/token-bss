use alloy::rpc::types::eth::Header;
use alloy::rpc::types::trace::geth::{GethTrace, GethDebugTracingCallOptions};

pub use alloy::{
    primitives::{
        Address, B256, U512, U256, U160, U128, Bytes, FixedBytes,
        utils as alloy_utils,
    },
    rpc::types::eth::{TransactionRequest, BlockNumberOrTag},
    network::{TransactionBuilder, Network},
    providers::Provider,
    transports::Transport,
};

pub use eyre::Result;

pub type TraceFn = Box<dyn Fn(
    TransactionRequest,
    Header, 
    GethDebugTracingCallOptions,
) -> Result<GethTrace, eyre::Report> + Send>;