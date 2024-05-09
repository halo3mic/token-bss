use std::str::FromStr;
use alloy::{
    rpc::types::eth::TransactionRequest,
    rpc::client::ClientRef,
    transports::Transport,
    providers::Provider,
    network::TransactionBuilder,
    primitives::{
        Address, B256, U256, Bytes,
        utils as alloy_utils
    },
};
use eyre::Result;


pub async fn set_balance<P, T>(
    provider: &P, 
    token: Address, 
    holder: Address,
    target_balance: f64, 
    slot_info: Option<(Address, B256, f64, String)>
) -> Result<U256> 
    where P: Provider<T>, T: Transport + Clone
{
    let (contract, slot, _update_ratio, lang) = match slot_info {
        Some(slot_info) => slot_info,
        None => {
            token_bss::find_slot(
                provider, 
                token,
                Some(holder), 
                None,
            ).await?
        }
    };
    let target_bal_fixed = token_dec_to_fixed(provider, token, target_balance).await?;
    let resulting_balance = update_balance(
        provider,
        token, 
        holder,
        target_bal_fixed,
        contract, 
        slot, 
        lang,
    ).await?;

    Ok(resulting_balance)
}

pub async fn update_balance<P, T>(
    provider: &P,
    token: Address,
    holder: Address,
    new_bal: U256,
    storage_contract: Address,
    slot: B256,
    lang_str: String,
) -> Result<U256> 
    where P: Provider<T>, T: Transport + Clone
{
    let map_loc = token_bss::EvmLanguage::from_str(&lang_str)?.mapping_loc(slot, holder);
    update_storage(&provider.client(), storage_contract, map_loc.into(), new_bal.into()).await?;
    let reflected_bal = call_balanceof(&provider, token, holder).await?;
    Ok(reflected_bal.into())
}

pub async fn update_storage<T>(
    client: &ClientRef<'_, T>, 
    contract: Address,
    slot: U256,
    value: B256
) -> Result<()> 
where T: Transport + Clone
{
    client
        .request(
            "anvil_setStorageAt", // Anvil has hardhat prefix as alias
            (contract, slot, value)
        )
        .await
        .map_err(|e| eyre::eyre!(format!("Storage update failed: {e:?}")))
        .and_then(|r| 
            if r {
                Ok(())
            } else {
                Err(eyre::eyre!("Did not update storage"))
            }
        )
}

// todo: reuse from slot_finder
pub async fn call_balanceof<P, T>(
    provider: &P,
    token: Address,
    holder: Address,
) -> Result<U256> 
    where P: Provider<T>, T: Transport + Clone
{
    let call_request = balanceof_call_req(holder, token)?;
    let balance = provider.call(&call_request).await?;
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

pub async fn token_dec_to_fixed<P, T>(
    provider: &P,
    token: Address,
    amount: f64,
) -> Result<U256> 
    where P: Provider<T>, T: Transport + Clone
{
    let dec = token_decimals(provider, token).await?;
    dec_to_fixed(amount, dec)
}

async fn token_decimals<P, T>(
    provider: &P,
    token: Address,
) -> Result<u8> 
    where P: Provider<T>, T: Transport + Clone
{
    let tx_req = TransactionRequest::default()
        .to(token)
        .with_input(Bytes::from_str("0x313ce567")?);
    let dec_bytes = provider.call(&tx_req).await?;
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
    use alloy::node_bindings::Anvil;
    use alloy::providers::ReqwestProvider;


    #[tokio::test]
    async fn test_token_dec_to_fixed() -> Result<()> {
        let provider_url = "https://arb1.arbitrum.io/rpc";
        let dec_amount = 23.434;
        let token = Address::from_str("0x912CE59144191C1204E64559FE8253a0e49E6548")?;
        let expected = alloy_utils::parse_ether(&dec_amount.to_string())?.into();

        let provider = ReqwestProvider::new_http(provider_url.parse()?);
        let fix_amount = token_dec_to_fixed(&provider, token, dec_amount).await?;

        assert_eq!(fix_amount, expected);
        Ok(())
    }

    #[tokio::test]
    async fn test_set_storage() -> Result<()> {
        let anvil = Anvil::new().fork("https://arb1.arbitrum.io/rpc").spawn();
        let provider = ReqwestProvider::new_http(anvil.endpoint_url());

        let desired_bal = U256::from(100);
        let token = Address::from_str("0xfa7f8980b0f1e64a2062791cc3b0871572f1f7f0")?;
        let holder = Address::from_str("0x1f9090aaE28b8a3dCeaDf281B0F12828e676c326")?;

        let map_loc = token_bss::EvmLanguage::Solidity.mapping_loc(
            B256::from(U256::wrapping_from(0x33)),
            holder,
        );

        update_storage(
            &provider.client(),
            token,
            map_loc.into(),
            B256::from(desired_bal),
        ).await?;

        let balance = call_balanceof(
            &provider,
            token,
            holder,
        ).await?;

        assert_eq!(balance, U256::from(100));

        Ok(())
    }


}