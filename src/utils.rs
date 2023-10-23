use ethers::{
    types::{H160, H256, U256, Bytes},
};
use hex::FromHex;
use ethers::abi::{
    ethereum_types::BigEndianHash,
    Tokenizable
};
use eyre::Result;
use ethers::utils::{Anvil, AnvilInstance};
use ethers::providers::{Http, Provider};
use ethers::types::transaction::eip2718::TypedTransaction;
use ethers::types::TransactionRequest;
use ethers::prelude::*;

pub fn bytes_to_h256(val: Bytes) -> H256 {
    let bytes = val.to_vec();
    if bytes.len() == 0 {
        H256::zero()
    } else {
        H256::from_slice(&bytes)
    }
}

fn bytes_to_u8(val: Bytes) -> u8 {
    let bytes = val.to_vec();
    if bytes.len() == 0 {
        0
    } else {
        bytes[bytes.len() - 1]
    }
}

pub fn u256_to_h160(val: U256) -> H160 {
    let mut bytes = [0; 32];
    val.to_big_endian(&mut bytes);
    H160::from_slice(&bytes[12..])
}

pub fn h256_to_h160(val: H256) -> H160 {
    H160::from_slice(&val.to_fixed_bytes()[12..])
}

pub fn u256_to_h256(val: U256) -> H256 {
    H256::from_uint(&val)
}

pub fn h256_to_u256(val: H256) -> U256 {
    H256::into_uint(&val)
}

pub fn spawn_anvil(fork_url: Option<&str>) -> AnvilInstance {
    (match fork_url {
        Some(url) => Anvil::new().fork(url),
        None => Anvil::new(),
    }).spawn()
}

pub fn spawn_anvil_provider(fork_url: Option<&str>) -> Result<(Provider<Http>, AnvilInstance)> {
    let anvil_fork = spawn_anvil(fork_url);
    let provider = Provider::<Http>::try_from(anvil_fork.endpoint())?;

    Ok((provider, anvil_fork))
}

pub async fn token_dec_to_fixed(
    provider: &Provider<Http>,
    token: H160,
    amount: f64,
) -> Result<U256> {
    let dec = token_decimals(provider, token).await?;
    dec_to_fixed(amount, dec)
}

fn dec_to_fixed(amount: f64, dec: u8) -> Result<U256> {
    let fixed = ethers::utils::parse_units(
        &amount.to_string(), 
        dec as u32
    )?;
    Ok(fixed.into())
}

async fn token_decimals(
    provider: &Provider<Http>,
    token: H160,
) -> Result<u8> {
    // let call_req = provider.request("eth_call", (serde_json::json!({
    //     "to": token,
    //     "data": data
    // }), "latest"))?;
    let call_req = TransactionRequest::new()
        .to(token)
        .data(Bytes::from_hex("0x313ce567")?);
    let dec_bytes = provider.call(&TypedTransaction::Legacy(call_req), None).await?;
    let dec = bytes_to_u8(dec_bytes);
    Ok(dec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethers::utils as eth_utils;

    #[test]
    fn test_u256_to_h160() {
        let val = U256::from_dec_str("520128635595255063220083964174648050700854198749").unwrap();
        let expected: H160 = "0x5b1b5fea1b99d83ad479df0c222f0492385381dd".parse().unwrap();
        assert_eq!(u256_to_h160(val), expected);
    }
}