
use alloy::{
    providers::RootProvider,
    transports::http::Http,
};
use reqwest::Client;

pub use alloy::{
    primitives::{
        Address, B256, U512, U256, U160, U128, Bytes, FixedBytes,
        utils as alloy_utils,
    },
    rpc::types::eth::{TransactionRequest, BlockId, BlockNumberOrTag},
    network::TransactionBuilder,
    providers::Provider,
};

pub type RootProviderHttp = RootProvider<Http<Client>>;

pub use eyre::Result;
