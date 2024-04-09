use ethers::{
    types::{H160, H256, U256, Bytes, transaction::eip2718::TypedTransaction, TransactionRequest},
    providers::{Http, Provider, Middleware},
    abi::ethereum_types::BigEndianHash,
    utils::{Anvil, AnvilInstance},
};
use hex::FromHex;
use eyre::Result;


pub mod conversion {
    use super::*;

    pub fn bytes_to_h256(val: Bytes) -> H256 {
        let bytes = val.to_vec();
        if bytes.len() == 0 {
            H256::zero()
        } else {
            H256::from_slice(&bytes[..32])
        }
    }
    
    pub fn bytes_to_u8(val: Bytes) -> u8 {
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
    
    pub fn u256_to_h256(val: U256) -> H256 {
        H256::from_uint(&val)
    }
    
    pub fn h256_to_u256(val: H256) -> U256 {
        H256::into_uint(&val)
    }
    
    pub fn h256_to_h160(val: H256) -> H160 {
        H160::from_slice(&val.to_fixed_bytes()[12..])
    }

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

pub fn env_var(var: &str) -> Result<String> {
    dotenv::dotenv().ok();
    std::env::var(var).map_err(|_| eyre::eyre!("{} not set", var))
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
    let dec = eth_call(
        provider, 
        token, 
        Bytes::from_hex("0x313ce567")?, 
        None
    ).await.map(conversion::bytes_to_u8)?;
    Ok(dec)
}

async fn eth_call(
    provider: &Provider<Http>, 
    to: H160, 
    data: Bytes, 
    gas: Option<u64>
) -> Result<Bytes> {
    let mut call_req = TransactionRequest::new()
        .to(to)
        .data(data);
    if let Some(gas) = gas {
        call_req = call_req.gas(gas);
    }
    let res = provider.call(&TypedTransaction::Legacy(call_req), None).await?;
    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_u256_to_h160() {
        let val = U256::from_dec_str("520128635595255063220083964174648050700854198749").unwrap();
        let expected: H160 = "0x5b1b5fea1b99d83ad479df0c222f0492385381dd".parse().unwrap();
        assert_eq!(conversion::u256_to_h160(val), expected);
    }

    #[tokio::test]
    async fn test_token_dec_to_fixed() -> Result<()> {
        let provider_url = "https://arb1.arbitrum.io/rpc";
        let dec_amount = 23.434;
        let token = H160::from_str("0x912CE59144191C1204E64559FE8253a0e49E6548")?;
        let expected = ethers::utils::parse_units(
            &dec_amount.to_string(), 
            18
        )?.into();

        let provider = Provider::<Http>::try_from(provider_url).unwrap();
        let fix_amount = token_dec_to_fixed(&provider, token, dec_amount).await?;

        assert_eq!(fix_amount, expected);
        Ok(())
    }


}