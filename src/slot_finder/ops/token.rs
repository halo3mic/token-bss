// ! Necessary to set gas for calls otherwise changing the wrong storage could 
// ! cause time-out eg. 0xf25c91c87e0b1fd9b4064af0f427157aab0193a7(Ethereum)

use alloy::rpc::types::eth::state::AccountOverride;
use std::collections::HashMap;
use super::super::utils;
use crate::common::*;


const BALANCEOF_4BYTE: &str = "0x70a08231";
const CALL_GAS_LIMIT: u128 = 200_000;

pub async fn call_balanceof(
    provider: &RootProviderHttp,
    call_request: &TransactionRequest,
) -> Result<U256> {
    let balance = provider.call(call_request, BlockId::latest()).await?;
    let balance = utils::bytes_to_u256(balance);
    Ok(balance)
}

pub async fn call_balanceof_with_storage_overrides(
    provider: &RootProviderHttp,
    call_request: &TransactionRequest,
    storage_contract: Address,
    map_loc: B256,
    new_slot_val: U256,
) -> Result<U256> {
    let state_diff: HashMap<_, _> = [(map_loc, new_slot_val)].into_iter().collect();
    let account_override = AccountOverride {
        state_diff: Some(state_diff),
        ..AccountOverride::default()
    };
    let state_override: HashMap<_, _> = 
        [(storage_contract, account_override)].into_iter().collect();

    let bal = provider.call_with_overrides(
        call_request, 
        BlockId::latest(), 
        state_override
    ).await?;
    Ok(utils::bytes_to_u256(bal))
}

pub fn balanceof_call_req(holder: Address, token: Address) -> Result<TransactionRequest> {
    let call_req = TransactionRequest::default()
        .with_gas_limit(CALL_GAS_LIMIT)
        .with_from(holder)
        .with_to(token.into())
        .with_input(balanceof_input_data(holder)?);
    Ok(call_req)
}

fn balanceof_input_data(holder: Address) -> Result<Bytes> {
    let holder = format!("{:?}", holder)[2..].to_string();
    let data_str = format!("{BALANCEOF_4BYTE}000000000000000000000000{holder}");
    let data = Bytes::from_hex(data_str)?;
    Ok(data)
}