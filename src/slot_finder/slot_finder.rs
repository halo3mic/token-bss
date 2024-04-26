use crate::common::*;
use super::{
    ops::{ storage, token, trace }, 
    trace_parser::TraceParser, 
    lang::EvmLanguage, 
    utils,
};


type SlotOutput = (Address, B256, f64, String);

pub async fn find_balance_slots_and_update_ratio(
    provider: &RootProviderHttp,
    holder: Address, 
    token: Address,
) -> Result<SlotOutput> {
    let slots = find_balance_slots(provider, holder, token).await?;
    closest_slot(provider, token, holder, slots).await
}

// ? if desired balance is already obtained skip the part below
pub async fn update_balance(
    provider: &RootProviderHttp, 
    token: Address,
    holder: Address,
    new_bal: U256,
    storage_contract: Address,
    slot: B256,
    lang_str: String,
) -> Result<U256> {
    let map_loc = EvmLanguage::from_str(&lang_str)?.mapping_loc(slot, holder);
    token::update_balance(&provider.client(), storage_contract, map_loc.into(), new_bal).await?;
    let reflected_bal = token::fetch_balanceof(&provider, token, holder).await?;
    Ok(reflected_bal.into())
}

pub async fn find_balance_slots(
    provider: &RootProviderHttp,
    holder: Address, 
    token: Address,
) -> Result<Vec<(Address, B256, EvmLanguage)>> {
    let tx_request = token::balanceof_call_req(holder, token)?;
    let response = trace::default_trace_call(provider, tx_request, None).await?;
    let matches = TraceParser::parse(response.struct_logs, token, holder)?;
    Ok(matches)
}

async fn closest_slot(
    provider: &RootProviderHttp,
    token: Address,
    holder: Address,
    slots: Vec<(Address, B256, EvmLanguage)>
) -> Result<SlotOutput, eyre::Error> {
    // Sequential execution necessary as there is only one anvil instance
    let d_one = |x: f64| (x - 1.0).abs();
    let mut closest_slot: Option<SlotOutput> = None;
    
    for (contract, slot, lang) in slots.into_iter() {
        let update_ratio_res = slot_update_to_bal_ratio(provider, token, contract, slot, holder, lang).await;
        
        if let Ok(ur) = update_ratio_res {
            match &closest_slot {
                Some((_, _, cr, _)) if d_one(*cr) < d_one(ur) => continue,
                _ => {
                    closest_slot = Some((contract, slot, ur, lang.to_string()));
                }
            }
        }
    }
    closest_slot.ok_or_else(|| eyre::eyre!("No valid slots found"))
}

// todo: instead of changing the storage just do eth_call with overrides
/// Check change in storage val is reflected in return value of balanceOf 
async fn slot_update_to_bal_ratio(
    provider: &RootProviderHttp, 
    token: Address,
    storage_contract: Address,
    slot: B256,
    holder: Address,
    lang: EvmLanguage,
) -> Result<f64> {
    let map_loc = lang.mapping_loc(slot, holder);
    let old_val = storage::get_storage_val(provider, storage_contract, map_loc.into()).await?;
    let new_slot_val: u128 = utils::rand_num();
    token::update_balance(&provider.client(), storage_contract, map_loc.into(), U256::from(new_slot_val)).await?;

    let res = if let Ok(new_bal) = token::fetch_balanceof(&provider, token, holder).await {
        if new_bal == B256::from(old_val) {
            return Err(eyre::eyre!("BalanceOf reflects old storage"));
        }
        let update_ratio = 
            utils::ratio_f64(
                new_bal.into(), 
                U256::from(new_slot_val),
                None
            );
        Ok(update_ratio)
    } else {
        Err(eyre::eyre!("BalanceOf failed"))  
    };
    // Change the storage value back to the original
    storage::anvil_update_storage(&provider.client(), storage_contract, map_loc.into(), old_val).await?;

    res
}


#[cfg(test)]
mod tests {
    use crate::utils;
    use super::*;

    fn rpc_endpoint() -> Result<String> {
        utils::env_var("RPC_URL")
    }


    #[tokio::test]
    async fn test_slot_finding() -> Result<()> {
        let (provider, _anvil_instance) = utils::spawn_anvil_provider(Some(&rpc_endpoint()?))?;

        let token: Address = "0xC011a73ee8576Fb46F5E1c5751cA3B9Fe0af2a6F".parse().unwrap();
        let holder: Address = "0x1f9090aaE28b8a3dCeaDf281B0F12828e676c326".parse().unwrap();

        let result = find_balance_slots(&provider, holder, token).await?;

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "0x5b1b5fea1b99d83ad479df0c222f0492385381dd".parse::<Address>().unwrap());
        assert_eq!(result[0].1, B256::from(U256::from(3)));
        Ok(())
    }

    #[tokio::test]
    async fn test_bal_storage_check_eth_usdc() -> Result<()> {
        let (provider, _anvil_instance) = utils::spawn_anvil_provider(Some(&rpc_endpoint()?))?;
        let token: Address = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".parse()?;
        let holder: Address = "0xDAFEA492D9c6733ae3d56b7Ed1ADB60692c98Bc5".parse().unwrap();
        let slot = U256::from(9).into();
        let lang = EvmLanguage::Solidity;

        let update_ratio = slot_update_to_bal_ratio(
            &provider, 
            token,
            token,
            slot, 
            holder,
            lang, 
        ).await?;
        
        assert_eq!(update_ratio, 1.0);
        Ok(())
    }

    #[tokio::test]
    async fn test_bal_storage_check_eth_sbtc() -> Result<()> {
        let (provider, _anvil_instance) = utils::spawn_anvil_provider(Some(&rpc_endpoint()?))?;
        let token: Address = "0xfE18be6b3Bd88A2D2A7f928d00292E7a9963CfC6".parse()?;
        let holder: Address = "0xDAFEA492D9c6733ae3d56b7Ed1ADB60692c98Bc5".parse().unwrap();
        let result = find_balance_slots(&provider, holder, token).await?;

        assert_eq!(result.len(), 1);
        let (contract, slot, lang) = result[0];
        let update_ratio = slot_update_to_bal_ratio(
            &provider, 
            token,
            contract, 
            slot, 
            holder,
            lang,
        ).await?;

        assert_eq!(update_ratio, 1.0);
        Ok(())
    }

    #[tokio::test]
    async fn test_bal_storage_check_eth_snx() -> Result<()> {
        let (provider, _anvil_instance) = utils::spawn_anvil_provider(Some(&rpc_endpoint()?))?;
        let token: Address = "0xC011a73ee8576Fb46F5E1c5751cA3B9Fe0af2a6F".parse()?;
        let holder: Address = "0xDAFEA492D9c6733ae3d56b7Ed1ADB60692c98Bc5".parse().unwrap();
        let result = find_balance_slots(&provider, holder, token).await?;

        assert_eq!(result.len(), 1);
        let (contract, slot, lang) = result[0];
        let update_ratio = slot_update_to_bal_ratio(
            &provider, 
            token,
            contract, 
            slot, 
            holder,
            lang,
        ).await?;

        assert_eq!(update_ratio, 1.0);
        Ok(())
    }

    #[tokio::test]
    async fn test_bal_storage_check_eth_stlink() -> Result<()> {
        let (provider, _anvil_instance) = utils::spawn_anvil_provider(Some(&rpc_endpoint()?))?;
        let token: Address = "0xb8b295df2cd735b15BE5Eb419517Aa626fc43cD5".parse()?;
        let holder: Address = "0xDAFEA492D9c6733ae3d56b7Ed1ADB60692c98Bc5".parse().unwrap();
        let result = find_balance_slots(&provider, holder, token).await?;

        assert_eq!(result.len(), 1);
        let (contract, slot, lang) = result[0];
        let ratio = slot_update_to_bal_ratio(
            &provider, 
            token,
            contract, 
            slot, 
            holder, 
            lang,
        ).await?;

        assert!(ratio > 1.);
        Ok(())
    }

    #[tokio::test]
    async fn test_bal_storage_check_eth_crv() -> Result<()> {
        let (provider, _anvil_instance) = utils::spawn_anvil_provider(Some(&rpc_endpoint()?))?;
        let token: Address = "0x6c3f90f043a72fa612cbac8115ee7e52bde6e490".parse()?;
        let holder: Address = "0xDAFEA492D9c6733ae3d56b7Ed1ADB60692c98Bc5".parse().unwrap();

        let result = find_balance_slots(&provider, holder, token).await?;
        let (contract, slot, lang) = result[0];
        let ratio = slot_update_to_bal_ratio(
            &provider, 
            token,
            contract, 
            slot, 
            holder,
            lang, 
        ).await?;

        assert_eq!(ratio, 1.0);
        Ok(())
    }

    #[tokio::test]
    async fn test_bal_storage_check_eth_basic() -> Result<()> {
        let (provider, _anvil_instance) = utils::spawn_anvil_provider(Some(&rpc_endpoint()?))?;
        let token: Address = "0xf25c91c87e0b1fd9b4064af0f427157aab0193a7".parse()?;
        let holder: Address = "0xDAFEA492D9c6733ae3d56b7Ed1ADB60692c98Bc5".parse().unwrap();
        let result = find_balance_slots(&provider, holder, token).await?;

        let (_c, s, r, _l) = closest_slot(&provider, token, holder, result).await?;
        
        assert_eq!(s, B256::from(U256::from(3)));
        assert_eq!(r, 1.0);
        Ok(())
    }

    #[tokio::test]
    async fn test_bal_storage_check_eurcv() -> Result<()> {
        let (provider, _anvil_instance) = utils::spawn_anvil_provider(Some(&rpc_endpoint()?))?;
        let token: Address = "0x5f7827fdeb7c20b443265fc2f40845b715385ff2".parse()?;
        let holder: Address = "0xDAFEA492D9c6733ae3d56b7Ed1ADB60692c98Bc5".parse().unwrap();
        let result = find_balance_slots(&provider, holder, token).await?;

        let (_c, s, r, _l) = closest_slot(&provider, token, holder, result).await?;
        
        assert_eq!(s, B256::from(U256::from(140)));
        assert_eq!(r, 1.0);
        Ok(())
    }

    #[tokio::test]
    async fn test_bal_storage_check_yv1inch() -> Result<()> {
        let (provider, _anvil_instance) = utils::spawn_anvil_provider(Some(&rpc_endpoint()?))?;
        let token: Address = "0xB8C3B7A2A618C552C23B1E4701109a9E756Bab67".parse()?;
        let holder: Address = "0xDAFEA492D9c6733ae3d56b7Ed1ADB60692c98Bc5".parse().unwrap();
        let result = find_balance_slots(&provider, holder, token).await?;
        println!("{:?}", result);
        let (_c, s, r, _l) = closest_slot(&provider, token, holder, result).await?;
        
        assert_eq!(s, B256::from(U256::from(3)));
        assert_eq!(r, 1.0);
        Ok(())
    }

}

