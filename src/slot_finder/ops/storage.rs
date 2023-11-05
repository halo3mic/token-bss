use ethers::prelude::*;
use eyre::Result;


pub async fn anvil_update_storage(
    provider: &Provider<Http>, 
    contract: H160,
    slot: H256,
    value: H256
) -> Result<()> {
    let res: bool = provider.request(
        "anvil_setStorageAt", 
        (contract, slot, value)
    ).await?;
    if res {
        Ok(())
    } else {
        Err(eyre::eyre!("Storage update failed"))
    }
}

pub async fn get_storage_val(
    provider: &Provider<Http>, 
    contract: H160,
    slot: H256,
) -> Result<H256> {
    let val = provider.get_storage_at(contract, slot, None).await?;
    Ok(val)
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::{self, conversion as c};

    #[tokio::test]
    async fn test_update_slot_val() -> Result<()> {
        let (provider, _anvil_instance) = utils::spawn_anvil_provider(None)?;

        let contract = H160::zero();
        let slot = c::u256_to_h256(U256::from(4));
        let new_val = c::u256_to_h256(U256::from(100));
        anvil_update_storage(&provider, contract, slot, new_val).await?;

        assert_eq!(get_storage_val(&provider, contract, slot).await?, new_val);
        Ok(())
    }
}