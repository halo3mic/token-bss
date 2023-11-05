use ethers::prelude::*;
use ethers::types::transaction::eip2718::TypedTransaction;
use ethers::utils::hex::FromHex;
use eyre::Result;
use crate::conversion as c;
use super::storage;


pub async fn update_balance(
    provider: &Provider<Http>, 
    storage_contract: H160,
    map_loc: H256,
    new_bal: U256,
) -> Result<()> {
    let new_bal = c::u256_to_h256(new_bal);
    storage::anvil_update_storage(provider, storage_contract, map_loc, new_bal).await?;
    Ok(())
}

pub async fn fetch_balanceof(
    provider: &Provider<Http>,
    token: H160, 
    holder: H160
) -> Result<H256> {
    let call_request = balanceof_call_req(holder, token)?;
    let mut tx = TypedTransaction::Legacy(call_request);
    tx.set_gas(100_000); // ! Necessary to set gas otherwise changing the wrong storage could incur a lot of processing eg. 0xf25c91c87e0b1fd9b4064af0f427157aab0193a7(Ethereum)
    let balance = provider.call(&tx, None).await?;
    let balance = c::bytes_to_h256(balance);
    Ok(balance)
}

pub fn balanceof_call_req(holder: H160, token: H160) -> Result<TransactionRequest> {
    let call_req = TransactionRequest::new()
        .from(holder)
        .to(token)
        .data(balanceof_input_data(holder)?);
    Ok(call_req)
}

fn balanceof_input_data(holder: H160) -> Result<Bytes> {
    // todo: use Bytes instead of strings
    let balanceof_4byte = "0x70a08231";
    let holder = format!("{:?}", holder)[2..].to_string();
    let data_str = format!("{balanceof_4byte}000000000000000000000000{holder}");
    let data = Bytes::from_hex(data_str)?;
    Ok(data)
}
