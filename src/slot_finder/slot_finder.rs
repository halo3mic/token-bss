// ! Works up to Foundry release `nightly-ca67d15f4abd46394b324c50e21e66f306a1162d`
use ethers::prelude::*;
use eyre::Result;
use super::{ 
    lang::EvmLanguage, 
    ops::{ storage, token, trace }, 
    trace_parser::TraceParser, 
    utils::{self, conversion as c},
};


type SlotOutput = (H160, H256, f64, String);

pub async fn find_balance_slots_and_update_ratio(
    provider: &Provider<Http>,
    holder: H160, 
    token: H160,
) -> Result<SlotOutput> {
    let slots = find_balance_slots(provider, holder, token).await?;
    first_valid_slot(provider, token, holder, slots).await
}

// ? if desired balance is already obtained skip the part below
pub async fn update_balance(
    provider: &Provider<Http>, 
    token: H160,
    holder: H160,
    new_bal: U256,
    storage_contract: H160,
    slot: H256,
    lang_str: String,
) -> Result<U256> {
    let map_loc = EvmLanguage::from_str(&lang_str)?.mapping_loc(slot, holder);
    token::update_balance(&provider, storage_contract, map_loc, new_bal).await?;
    let reflected_bal = token::fetch_balanceof(&provider, token, holder).await?;
    Ok(c::h256_to_u256(reflected_bal))
}

pub async fn find_balance_slots(
    provider: &Provider<Http>,
    holder: H160, 
    token: H160,
) -> Result<Vec<(H160, H256, EvmLanguage)>> {
    let tx_request = token::balanceof_call_req(holder, token)?;
    let response = trace::default_trace_call(provider, tx_request, None).await?;
    let matches = TraceParser::parse(response.struct_logs, token, holder)?;
    Ok(matches)
}

async fn first_valid_slot(
    provider: &Provider<Http>,
    token: H160,
    holder: H160,
    slots: Vec<(H160, H256, EvmLanguage)>
) -> Result<SlotOutput> {
    for (contract, slot, lang) in slots {
        if let Ok(update_ratio) = 
            slot_update_to_bal_ratio(
                &provider, 
                token, 
                contract, 
                slot, 
                holder, 
                lang
            ).await 
        {
            return Ok((contract, slot, update_ratio, lang.to_string()));
        }
    }
    return Err(eyre::eyre!("No valid slots found"));
}


/// Check change in storage val is reflected in return value of balanceOf 
async fn slot_update_to_bal_ratio(
    provider: &Provider<Http>, 
    token: H160,
    storage_contract: H160,
    slot: H256,
    holder: H160,
    lang: EvmLanguage,
) -> Result<f64> {
    let map_loc = lang.mapping_loc(slot, holder);
    let old_val = storage::get_storage_val(provider, storage_contract, map_loc).await?;
    let new_slot_val: u128 = utils::rand_num();
    token::update_balance(&provider, storage_contract, map_loc, U256::from(new_slot_val)).await?;

    let res = if let Ok(new_bal) = token::fetch_balanceof(&provider, token, holder).await {
        if new_bal == old_val {
            return Err(eyre::eyre!("BalanceOf reflects old storage"));
        }
        let update_ratio = 
            utils::ratio_f64(
                c::h256_to_u256(new_bal), 
                U256::from(new_slot_val),
                None
            );
        Ok(update_ratio)
    } else {
        Err(eyre::eyre!("BalanceOf failed"))  
    };
    // Change the storage value back to the original
    storage::anvil_update_storage(provider, storage_contract, map_loc, old_val).await?;

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

        let token: H160 = "0xC011a73ee8576Fb46F5E1c5751cA3B9Fe0af2a6F".parse().unwrap();
        let holder: H160 = "0x1f9090aaE28b8a3dCeaDf281B0F12828e676c326".parse().unwrap();

        let result = find_balance_slots(&provider, holder, token).await?;

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "0x5b1b5fea1b99d83ad479df0c222f0492385381dd".parse::<H160>().unwrap());
        assert_eq!(result[0].1, c::u256_to_h256(U256::from(3)));
        Ok(())
    }

    #[tokio::test]
    async fn test_bal_storage_check_eth_usdc() -> Result<()> {
        let (provider, _anvil_instance) = utils::spawn_anvil_provider(Some(&rpc_endpoint()?))?;
        let token: H160 = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".parse()?;
        let holder: H160 = "0xDAFEA492D9c6733ae3d56b7Ed1ADB60692c98Bc5".parse().unwrap();
        let slot = c::u256_to_h256(U256::from(9));
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
        let token: H160 = "0xfE18be6b3Bd88A2D2A7f928d00292E7a9963CfC6".parse()?;
        let holder: H160 = "0xDAFEA492D9c6733ae3d56b7Ed1ADB60692c98Bc5".parse().unwrap();
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
        let token: H160 = "0xC011a73ee8576Fb46F5E1c5751cA3B9Fe0af2a6F".parse()?;
        let holder: H160 = "0xDAFEA492D9c6733ae3d56b7Ed1ADB60692c98Bc5".parse().unwrap();
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
        let token: H160 = "0xb8b295df2cd735b15BE5Eb419517Aa626fc43cD5".parse()?;
        let holder: H160 = "0xDAFEA492D9c6733ae3d56b7Ed1ADB60692c98Bc5".parse().unwrap();
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
        let token: H160 = "0x6c3f90f043a72fa612cbac8115ee7e52bde6e490".parse()?;
        let holder: H160 = "0xDAFEA492D9c6733ae3d56b7Ed1ADB60692c98Bc5".parse().unwrap();

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
        let token: H160 = "0xf25c91c87e0b1fd9b4064af0f427157aab0193a7".parse()?;
        let holder: H160 = "0xDAFEA492D9c6733ae3d56b7Ed1ADB60692c98Bc5".parse().unwrap();
        let result = find_balance_slots(&provider, holder, token).await?;

        let (_c, s, r, _l) = first_valid_slot(&provider, token, holder, result).await?;
        
        assert_eq!(s, c::u256_to_h256(U256::from(3)));
        assert_eq!(r, 1.0);
        Ok(())
    }

    #[tokio::test]
    async fn test_bal_storage_check_eurcv() -> Result<()> {
        let (provider, _anvil_instance) = utils::spawn_anvil_provider(Some(&rpc_endpoint()?))?;
        let token: H160 = "0x5f7827fdeb7c20b443265fc2f40845b715385ff2".parse()?;
        let holder: H160 = "0xDAFEA492D9c6733ae3d56b7Ed1ADB60692c98Bc5".parse().unwrap();
        let result = find_balance_slots(&provider, holder, token).await?;

        let (_c, s, r, _l) = first_valid_slot(&provider, token, holder, result).await?;
        
        assert_eq!(s, c::u256_to_h256(U256::from(140)));
        assert_eq!(r, 1.0);
        Ok(())
    }

}

