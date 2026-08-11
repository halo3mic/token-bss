// ! Necessary to set gas for calls otherwise changing the wrong storage could 
// ! cause time-out eg. 0xf25c91c87e0b1fd9b4064af0f427157aab0193a7(Ethereum)

use alloy::rpc::types::eth::state::AccountOverride;
use std::{collections::HashMap, str::FromStr};
use super::super::utils;
use crate::common::*;


const BALANCEOF_4BYTE: &str = "0x70a08231";
const ALLOWANCE_4BYTE: &str = "0xdd62ed3e";
const CALL_GAS_LIMIT: u128 = 200_000;

fn view_call_req(from: Address, to: Address, input: Bytes) -> TransactionRequest {
    // Zero fees so eth_call / debug_traceCall don't fail on OP-stack forks when
    // `from` has no native balance (e.g. Unichain Anvil with --optimism).
    TransactionRequest::default()
        .with_input(input)
        .with_gas_limit(CALL_GAS_LIMIT)
        .with_max_fee_per_gas(0)
        .with_max_priority_fee_per_gas(0)
        .with_gas_price(0)
        .with_from(from)
        .with_to(to.into())
}

pub async fn call_request<P, T>(
    provider: &P,
    call_request: &TransactionRequest,
) -> Result<U256> 
    where P: Provider<T>, T: Transport + Clone
{
    let balance = provider.call(call_request).await?;
    let balance = utils::bytes_to_u256(balance);
    Ok(balance)
}

pub async fn call_request_with_storage_overrides<P, T>(
    provider: &P,
    call_request: &TransactionRequest,
    storage_contract: Address,
    map_loc: B256,
    new_slot_val: B256,
) -> Result<U256> 
    where P: Provider<T>, T: Transport + Clone
{
    let state_diff: HashMap<_, _> = [(map_loc, new_slot_val)].into_iter().collect();
    let account_override = AccountOverride {
        state_diff: Some(state_diff),
        ..AccountOverride::default()
    };
    let state_override: HashMap<_, _> = 
        [(storage_contract, account_override)].into_iter().collect();

    let bal = provider.call(call_request).overrides(&state_override).await?;
    Ok(utils::bytes_to_u256(bal))
}

pub fn balanceof_call_req(holder: Address, token: Address) -> Result<TransactionRequest> {
    Ok(view_call_req(holder, token, balanceof_input_data(holder)?))
}

pub fn allowance_call_req(owner: Address, spender: Address, token: Address) -> Result<TransactionRequest> {
    Ok(view_call_req(owner, token, allowance_input_data(owner, spender)?))
}

fn balanceof_input_data(holder: Address) -> Result<Bytes> {
    let holder = format!("{:?}", holder)[2..].to_string();
    let data_str = format!("{BALANCEOF_4BYTE}000000000000000000000000{holder}");
    let data = Bytes::from_str(&data_str)?;
    Ok(data)
}

fn allowance_input_data(owner: Address, spender: Address) -> Result<Bytes> {
    let owner = format!("{:?}", owner)[2..].to_string();
    let spender = format!("{:?}", spender)[2..].to_string();
    let data_str = format!(
        "{ALLOWANCE_4BYTE}000000000000000000000000{owner}000000000000000000000000{spender}"
    );
    let data = Bytes::from_str(&data_str)?;
    Ok(data)
}