use std::str::FromStr;
use reqwest::Client;
use alloy::{
    rpc::types::eth::{TransactionRequest, BlockId},
    transports::http::Http,
    providers::{Provider, RootProvider},
    network::TransactionBuilder,
    primitives::{
        Address, B256, U256, Bytes,
        utils as alloy_utils,
    },
};
use eyre::Result;

type RootProviderHttp = RootProvider<Http<Client>>;


pub async fn set_balance(
    provider_url: &str, 
    token: Address, 
    holder: Address,
    target_balance: f64, 
    slot_info: Option<(Address, B256, f64, String)>
) -> Result<U256> {
    let (contract, slot, _update_ratio, lang) = match slot_info {
        Some(slot_info) => slot_info,
        None => {
            erc20_topup::find_slot(
                &provider_url, 
                token,
                Some(holder), 
            ).await?
        }
    };
    let provider = http_provider_from_url(provider_url);
    let target_bal_fixed = token_dec_to_fixed(&provider, token, target_balance).await?;
    let resulting_balance = update_balance(
        &provider,
        provider_url,
        token, 
        holder,
        target_bal_fixed,
        contract, 
        slot, 
        lang,
    ).await?;

    Ok(resulting_balance)
}

pub async fn update_balance(
    provider: &RootProviderHttp,
    provider_url: &str,
    token: Address,
    holder: Address,
    new_bal: U256,
    storage_contract: Address,
    slot: B256,
    lang_str: String,
) -> Result<U256> {
    let map_loc = erc20_topup::EvmLanguage::from_str(&lang_str)?.mapping_loc(slot, holder);
    update_storage(&provider_url, storage_contract, map_loc.into(), new_bal.into()).await?;
    let reflected_bal = call_balanceof(&provider, token, holder).await?;
    Ok(reflected_bal.into())
}

pub async fn update_storage(
    provider_url: &str, 
    contract: Address,
    slot: U256,
    value: B256
) -> Result<()> {
    // todo: update this once alloy's client supports setStorageAt return
    // client.request(
    //     "hardhat_setStorageAt", // Anvil has hardhat prefix as alias
    //     (contract, slot, value)
    // ).await.map_err(|e| { 
    //     eyre::eyre!(format!("Storage update failed: {e:?}"))
    // })
    let rpc_request = r#"{
        "jsonrpc": "2.0",
        "method": "hardhat_setStorageAt",
        "params": [
            ""#.to_owned() + &format!("{contract:}") + r#"",
            ""# + &format!("{slot:}") + r#"",
            ""# + &format!("{value:}") + r#""
        ],
        "id": 1
    }"#;
    let response = reqwest::Client::new()
        .post(provider_url)
        .body(rpc_request)
        .header("Content-Type", "application/json")
        .send()
        .await?;
    let response = response.json::<jsonrpc::Response>().await?;
    response.check_error()
        .map_err(|e| eyre::eyre!(format!("Storage update failed: {e:?}")))
}

fn http_provider_from_url(url: &str) -> RootProviderHttp {
    RootProviderHttp::new_http(url.parse().unwrap())
}

// todo: reuse from slot_finder
pub async fn call_balanceof(
    provider: &RootProviderHttp,
    token: Address,
    holder: Address,
) -> Result<U256> {
    let call_request = balanceof_call_req(holder, token)?;
    let balance = provider.call(&call_request, BlockId::latest()).await?;
    let balance = bytes_to_u256(balance);
    Ok(balance)
}

pub fn balanceof_call_req(holder: Address, token: Address) -> Result<TransactionRequest> {
    let call_req = TransactionRequest::default()
        .with_input(balanceof_input_data(holder)?)
        .with_from(holder)
        .with_to(token.into());
    Ok(call_req)
}

fn balanceof_input_data(holder: Address) -> Result<Bytes> {
    const BALANCEOF_4BYTE: &str = "0x70a08231";
    let holder = format!("{:?}", holder)[2..].to_string();
    let data_str = format!("{BALANCEOF_4BYTE}000000000000000000000000{holder}");
    let data = Bytes::from_str(&data_str)?;
    Ok(data)
}

pub async fn token_dec_to_fixed(
    provider: &RootProviderHttp,
    token: Address,
    amount: f64,
) -> Result<U256> {
    let dec = token_decimals(provider, token).await?;
    dec_to_fixed(amount, dec)
}

async fn token_decimals(
    provider: &RootProviderHttp,
    token: Address,
) -> Result<u8> {
    let tx_req = TransactionRequest::default()
        .to(token)
        .with_input(Bytes::from_str("0x313ce567")?);
    let dec_bytes = provider.call(&tx_req, BlockId::latest()).await?;
    let dec = bytes_to_u8(dec_bytes);
    Ok(dec)
}

fn dec_to_fixed(amount: f64, dec: u8) -> Result<U256> {
    Ok(alloy_utils::parse_units(
        &amount.to_string(), 
        dec
    )?.into())
}

fn bytes_to_u256(val: Bytes) -> U256 {
    let bytes = val.to_vec();
    if bytes.len() == 0 {
        U256::ZERO
    } else {
        B256::from_slice(&bytes[..32]).into()
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[tokio::test]
    async fn test_token_dec_to_fixed() -> Result<()> {
        let provider_url = "https://arb1.arbitrum.io/rpc";
        let dec_amount = 23.434;
        let token = Address::from_str("0x912CE59144191C1204E64559FE8253a0e49E6548")?;
        let expected = alloy_utils::parse_ether(&dec_amount.to_string())?.into();

        let provider = RootProviderHttp::new_http(provider_url.parse()?);
        let fix_amount = token_dec_to_fixed(&provider, token, dec_amount).await?;

        assert_eq!(fix_amount, expected);
        Ok(())
    }


}