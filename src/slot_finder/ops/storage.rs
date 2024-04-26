use alloy::{
    primitives::{Address, U256},
    rpc::types::eth::BlockId,
    rpc::client::ClientRef,
    transports::Transport,
    providers::Provider,
    network::Ethereum,
};
use eyre::Result;


pub async fn anvil_update_storage<T: Clone + Transport>(
    client: &ClientRef<'_, T>, 
    contract: Address,
    slot: U256,
    value: U256
) -> Result<()> {
    let res: bool = client.request(
        "anvil_setStorageAt", 
        (contract, slot, value)
    ).await?;
    if res {
        Ok(())
    } else {
        Err(eyre::eyre!("Storage update failed"))
    }
}

pub async fn get_storage_val<T: Clone + Transport>(
    provider: &impl Provider<T, Ethereum>, 
    contract: Address,
    key: U256,
) -> Result<U256> {
    let val = provider.get_storage_at(contract, key, BlockId::latest()).await?;
    Ok(val)
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils;

    #[tokio::test]
    async fn test_update_slot_val() -> Result<()> {
        let (provider, _anvil_instance) = utils::spawn_anvil_provider(None)?;

        let contract = Address::ZERO;
        let slot = U256::from(4);
        let new_val = U256::from(100);
        anvil_update_storage(&provider.client(), contract, slot, new_val).await?;

        assert_eq!(get_storage_val(&provider, contract, slot).await?, new_val);
        Ok(())
    }
}