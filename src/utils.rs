use crate::common::*;

// todo: does this really need to be here?


pub async fn token_dec_to_fixed(
    provider: &RootProviderHttp,
    token: Address,
    amount: f64,
) -> Result<U256> {
    let dec = token_decimals(provider, token).await?;
    dec_to_fixed(amount, dec)
}

fn dec_to_fixed(amount: f64, dec: u8) -> Result<U256> {
    Ok(alloy_utils::parse_units(
        &amount.to_string(), 
        dec
    )?.into())
}

async fn token_decimals(
    provider: &RootProviderHttp,
    token: Address,
) -> Result<u8> {
    let tx_req = TransactionRequest::default()
        .to(token)
        .with_input(Bytes::from_hex("0x313ce567")?);
    let dec_bytes = provider.call(&tx_req, BlockId::latest()).await?;
    let dec = bytes_to_u8(dec_bytes);
    Ok(dec)
}

fn bytes_to_u8(val: Bytes) -> u8 {
    let bytes = val.to_vec();
    if bytes.len() == 0 {
        0
    } else {
        bytes[bytes.len() - 1]
    }
}

// // todo: after using eth_call with overrides it doesn't make sens to have this here -> move to utils
// // ? use update_ratio to set a target balance (if fee is 2% take it into account)
// pub async fn set_balance(
//     provider_url: &str, 
//     token: Address, 
//     holder: Address,
//     target_balance: f64, 
//     slot_info: Option<(Address, B256, f64, String)>
// ) -> Result<U256> {
//     let provider = http_provider_from_url(provider_url);
//     let (contract, slot, _update_ratio, lang) = match slot_info {
//         Some(slot_info) => slot_info,
//         None => {
//             slot_finder::find_balance_slots_and_update_ratio(
//                 &provider, 
//                 holder, 
//                 token
//             ).await?
//         }
//     };
//     let target_bal_fixed = utils::token_dec_to_fixed(&provider, token, target_balance).await?;
//     let resulting_balance = slot_finder::update_balance(
//         &provider, 
//         token, 
//         holder,
//         target_bal_fixed,
//         contract, 
//         slot, 
//         lang,
//     ).await?;

//     Ok(resulting_balance)
// }


// // // ? if desired balance is already obtained skip the part below
// pub async fn update_balance(
//     provider: &RootProviderHttp, 
//     token: Address,
//     holder: Address,
//     new_bal: U256,
//     storage_contract: Address,
//     slot: B256,
//     lang_str: String,
// ) -> Result<U256> {
//     let map_loc = EvmLanguage::from_str(&lang_str)?.mapping_loc(slot, holder);
//     update_balance(&provider.client(), storage_contract, map_loc.into(), new_bal).await?;
//     let reflected_bal = token::fetch_balanceof(&provider, token, holder).await?;
//     Ok(reflected_bal.into())
// }

// pub async fn anvil_update_storage<T: Clone + Transport>(
//     client: &ClientRef<'_, T>, 
//     contract: Address,
//     slot: U256,
//     value: U256
// ) -> Result<()> {
//     client.request(
//         "anvil_setStorageAt", // todo: what if hardhat is used?
//         (contract, slot, value)
//     ).await.map_err(|e| { 
//         eyre::eyre!(format!("Storage update failed: {e:?}"))
//     })
// }

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