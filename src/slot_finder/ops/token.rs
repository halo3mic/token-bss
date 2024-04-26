use crate::common::*;
use super::storage;
use super::super::utils;


const BALANCEOF_4BYTE: &str = "0x70a08231";
const CALL_GAS_LIMIT: u128 = 200_000;

// todo: so many helpers really needed?
pub async fn update_balance<T: Clone + Transport>(
    client: &ClientRef<'_, T>, 
    storage_contract: Address,
    map_loc: U256,
    new_bal: U256,
) -> Result<()> {
    storage::anvil_update_storage(client, storage_contract, map_loc, new_bal).await?;
    Ok(())
}

pub async fn fetch_balanceof(
    provider: &RootProviderHttp,
    token: Address, 
    holder: Address
) -> Result<U256> {
    let mut call_request = balanceof_call_req(holder, token)?;
    call_request.set_gas_limit(CALL_GAS_LIMIT); // ! Necessary to set gas otherwise changing the wrong storage could incur a lot of processing eg. 0xf25c91c87e0b1fd9b4064af0f427157aab0193a7(Ethereum)
    let balance = provider.call(&call_request, BlockId::latest()).await?;
    let balance = utils::bytes_to_u256(balance);
    Ok(balance)
}

pub fn balanceof_call_req(holder: Address, token: Address) -> Result<TransactionRequest> {
    let call_req = TransactionRequest::default()
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