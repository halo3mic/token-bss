use futures::future::join_all;
use super::{
    trace_parser::TraceParser, 
    ops::{token, trace}, 
    lang::EvmLanguage, 
    utils,
};
use crate::common::*;


// todo: use a struct
type SlotOutput = (Address, B256, f64, String);

pub async fn find_balance_slots_and_update_ratio(
    provider: &RootProviderHttp,
    holder: Address, 
    token: Address,
) -> Result<SlotOutput> {
    let slots = find_balance_slots(provider, holder, token).await?;
    closest_slot(provider, token, holder, slots).await
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

// Note this would choose 0 over 2
async fn closest_slot(
    provider: &RootProviderHttp,
    token: Address,
    holder: Address,
    slots: Vec<(Address, B256, EvmLanguage)>
) -> Result<SlotOutput, eyre::Error> {
    let d_one = |x: f64| ((x - 1.0).abs() * 100.) as u8;
    let future_results = join_all(slots.into_iter()
        .map(|(c, s, la)| async move {
            slot_update_to_bal_ratio(provider, token, c,s, holder, la)
                .await
                .map(|ur| (c, s, ur, la.to_string()))
        })
    );
    future_results.await
        .into_iter()
        .filter_map(|x| x.ok())
        .min_by_key(|x| d_one(x.2))
        .ok_or_else(|| eyre::eyre!("No valid slots found"))
}

// todo: too many params
// todo: more suiting name
async fn slot_update_to_bal_ratio(
    provider: &RootProviderHttp, 
    token: Address,
    storage_contract: Address,
    slot: B256,
    holder: Address,
    lang: EvmLanguage,
) -> Result<f64> {
    let new_slot_val = U256::from(rand::random::<u128>()); // todo: In scenario where this is excatly the same as the current balance it fails
    let map_loc = lang.mapping_loc(slot, holder);
    let call_request = token::balanceof_call_req(holder, token)?;

    let override_bal_future = token::call_request_with_storage_overrides(
        provider,
        &call_request,
        storage_contract,
        map_loc,
        new_slot_val,
    );
    let real_bal_future = token::call_request(provider, &call_request);
    let (override_bal, real_bal) = tokio::try_join!(override_bal_future, real_bal_future)?;

    if override_bal == real_bal {
        return Err(eyre::eyre!("Balance not updated"));
    }
    let update_ratio = utils::ratio_f64(override_bal, new_slot_val, None);
    
    Ok(update_ratio)
}


#[cfg(test)]
mod tests {
    use alloy::node_bindings::{Anvil, AnvilInstance};
    use super::*;

    pub fn spawn_anvil_provider(fork_url: Option<&str>) -> Result<(RootProviderHttp, AnvilInstance)> {
        let anvil_fork = spawn_anvil(fork_url);
        let provider = RootProviderHttp::new_http(anvil_fork.endpoint().parse()?);
    
        Ok((provider, anvil_fork))
    }
    
    pub fn spawn_anvil(fork_url: Option<&str>) -> AnvilInstance {
        (match fork_url {
            Some(url) => Anvil::new().fork(url),
            None => Anvil::new(),
        }).spawn()
    } 
    
    pub fn env_var(var: &str) -> Result<String> {
        dotenv::dotenv().ok();
        std::env::var(var).map_err(|_| eyre::eyre!("{} not set", var))
    }

    fn rpc_endpoint() -> Result<String> {
        env_var("RPC_URL") // todo: rename this to test-rpc or emphasize that it must be eth based
    }

    #[tokio::test]
    async fn test_slot_finding() -> Result<()> {
        let (provider, _anvil_instance) = spawn_anvil_provider(Some(&rpc_endpoint()?))?;

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
        let (provider, _anvil_instance) = spawn_anvil_provider(Some(&rpc_endpoint()?))?;
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
        let (provider, _anvil_instance) = spawn_anvil_provider(Some(&rpc_endpoint()?))?;
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
        let (provider, _anvil_instance) = spawn_anvil_provider(Some(&rpc_endpoint()?))?;
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
        let (provider, _anvil_instance) = spawn_anvil_provider(Some(&rpc_endpoint()?))?;
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
        let (provider, _anvil_instance) = spawn_anvil_provider(Some(&rpc_endpoint()?))?;
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
        let (provider, _anvil_instance) = spawn_anvil_provider(Some(&rpc_endpoint()?))?;
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
        let (provider, _anvil_instance) = spawn_anvil_provider(Some(&rpc_endpoint()?))?;
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
        let (provider, _anvil_instance) = spawn_anvil_provider(Some(&rpc_endpoint()?))?;
        let token: Address = "0xB8C3B7A2A618C552C23B1E4701109a9E756Bab67".parse()?;
        let holder: Address = "0xDAFEA492D9c6733ae3d56b7Ed1ADB60692c98Bc5".parse().unwrap();
        let result = find_balance_slots(&provider, holder, token).await?;
        let (_c, s, r, _l) = closest_slot(&provider, token, holder, result).await?;
        
        assert_eq!(s, B256::from(U256::from(3)));
        assert_eq!(r, 1.0);
        Ok(())
    }

}

