// todo: check which tokens are supported
// todo: change token balance
// todo: usage from cli

use ethers::prelude::*;
use ethers::providers::{Provider, Http};
use ethers::types::{GethDebugTracingCallOptions, GethTrace, GethTraceFrame, StructLog, GethDebugTracingOptions};
use ethers::utils::hex::FromHex;
use ethers::abi::Tokenizable;
use eyre::Result;
use std::collections::HashMap;
use ethers::types::transaction::eip2718::TypedTransaction;
use super::utils;
use rand;


const balanceof_4byte: &str = "0x70a08231";

pub async fn find_balance_slots(
    provider: Provider<Http>,
    holder: H160, 
    token: H160,
) -> Result<Vec<(H160, U256)>> {
    let tx_request = balanceof_call_req(holder, token)?;
    let response = default_trace_call(provider, tx_request, None).await?;
    let return_val = utils::bytes_to_h256(response.return_value);
    let matches = find_slot(response.struct_logs, holder, token, &return_val)?;

    Ok(matches)
}

fn balanceof_call_req(holder: H160, token: H160) -> Result<TransactionRequest> {
    Ok(TransactionRequest::new()
        .from(holder)
        .to(token)
        .data(balanceof_input_data(holder)?)
    )
}

fn balanceof_input_data(holder: H160) -> Result<Bytes> {
    let holder = format!("{:?}", holder)[2..].to_string();
    let data_str = format!("{balanceof_4byte}000000000000000000000000{holder}");
    let data = Bytes::from_hex(data_str)?;
    Ok(data)
}


/// Check change in storage val is reflected in return value of balanceOf 
async fn check_slot_links_to_balanceof(
    provider: &Provider<Http>, 
    token: H160,
    slot: H256,
    holder: H160,
) -> Result<()> {
    let map_loc = mapping_location(&slot, &H256::from(holder));
    let old_val = get_storage_val(provider, token, map_loc).await?;
    let new_val = utils::u256_to_h256(U256::from(rand::random::<u64>()));
    anvil_update_storage(provider, token, map_loc, new_val).await?;
    let call_request = TypedTransaction::Legacy(balanceof_call_req(holder, token)?);
    let balance = provider.call(&call_request, None).await?;
    let balance = utils::bytes_to_h256(balance);
    anvil_update_storage(provider, token, map_loc, old_val).await?; // Change the storage value back to the original

    if balance == new_val {
        Ok(())
    } else {
        Err(eyre::eyre!("BalanceOf does not reflect storage change"))
    }
}

pub fn mapping_location(storage_index: &H256, key: &H256) -> H256 {
    ethers::utils::keccak256(ethers::abi::encode(&[
        key.into_token(), 
        storage_index.into_token()
    ])).into()
}

async fn anvil_update_storage(
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

async fn get_storage_val(
    provider: &Provider<Http>, 
    contract: H160,
    slot: H256,
) -> Result<H256> {
    let val = provider.get_storage_at(contract, slot, None).await?;
    Ok(val)
}

async fn default_trace_call(
    provider: Provider<Http>, 
    call_request: TransactionRequest, 
    block: Option<BlockId>
) -> Result<DefaultFrame> {
    let tracing_options = GethDebugTracingOptions {
        disable_storage: Some(false),
        disable_stack: Some(false),
        enable_memory: Some(true),
        enable_return_data: Some(true),
        tracer: None,
        tracer_config: None,
        timeout: None,
    };
    let call_options = GethDebugTracingCallOptions {
        tracing_options: tracing_options,
        state_overrides: None,
    };
    let response = provider.debug_trace_call(
        call_request, 
        block, 
        call_options
    ).await?;

    match response {
        GethTrace::Known(GethTraceFrame::Default(trace)) => {
            if trace.failed {
                Err(eyre::eyre!("traceCall failed"))
            } else {
                Ok(trace)
            }
        }
        _ => Err(eyre::eyre!("Only known default traces supported"))
    }
}

fn find_slot(
    struct_logs: Vec<StructLog>, 
    holder: H160, 
    token: H160, 
    return_val: &H256
) -> Result<Vec<(H160, U256)>> {
    let mut depth_to_address = HashMap::new();
    depth_to_address.insert(1, token);
    let mut results = Vec::new();
    
    for log in struct_logs {
        let depth = log.depth;
        
        if log.op == "SLOAD" {
            // println!("{:#?}", log.clone());
            if log.memory.as_ref().map(|m|m.len() < 2).unwrap_or(true) || log.stack.as_ref().is_none() || log.storage.as_ref().is_none() {
                continue;
            }
            let memory_content = log.memory.expect("SLOAD should have memory content");
                  
            if let Ok(sload_address) = format!("0x{}", memory_content[0].trim_start_matches("0")).parse::<H160>() {
                // Find SLOAD operations containing the holder address
                if sload_address == holder {
                    // println!("hashed val: {:?}", format!("0x{}{}", memory_content[0], memory_content[1]));
                    let hashed = Bytes::from_hex(format!("0x{}{}", memory_content[0], memory_content[1])).unwrap();
                    let hash = ethers::utils::keccak256(hashed);
                    let hhash = H256::from(hash);
                    let uhash = U256::from(hash);

                    // println!("Hash: {:#?}; UHash: {:#?}", hhash.to_string(), uhash);
    
                    // Return the slot if its hash with holder address is mapped to return value in storage and hash is on top of stack
                    if log.stack.unwrap().last().unwrap() == &uhash && log.storage.unwrap().get(&hhash).map(|v| v == return_val).unwrap_or(false) {
                        let slot = U256::from_str_radix(&memory_content[1][2..], 16)?;
                        let contract = depth_to_address.get(&depth).unwrap();
                        results.push((*contract, slot));
                    }
                }
            }
        } else if log.op == "STATICCALL" {
            let stack = log.stack.unwrap();
            let address = H160::from_low_u64_be(stack[4].low_u64());
            depth_to_address.insert(depth + 1, address);
        } else if log.op == "DELEGATECALL" {
            let prev_address = *depth_to_address.get(&depth).unwrap();
            depth_to_address.insert(depth + 1, prev_address);
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use ethers::utils::{AnvilInstance, Anvil};

    pub fn spawn_anvil_provider(fork_url: Option<&str>) -> Result<(Provider<Http>, AnvilInstance)> {
        let anvil_fork = 
            (match fork_url {
                Some(url) => Anvil::new().fork(url),
                None => Anvil::new(),
            }).spawn();
        let provider = Provider::<Http>::try_from(anvil_fork.endpoint())?;

        Ok((provider, anvil_fork))
    }

    #[tokio::test]
    async fn test_slot_finding() -> Result<()> {
        let config = Config::from_env()?;
        let (provider, _anvil_instance) = spawn_anvil_provider(Some(&config.eth_rpc_endpoint))?;

        let holder: H160 = "0xDAFEA492D9c6733ae3d56b7Ed1ADB60692c98Bc5".parse().unwrap();
        let token: H160 = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".parse().unwrap();

        let result = find_balance_slots(provider, holder, token).await?;

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, token);
        assert_eq!(result[0].1, U256::from(9));

        Ok(())
    }

    #[tokio::test]
    async fn test_update_slot_val() -> Result<()> {
        let (provider, _anvil_instance) = spawn_anvil_provider(None)?;

        let contract = H160::zero();
        let slot = utils::u256_to_h256(U256::from(4));
        let new_val = utils::u256_to_h256(U256::from(100));
        anvil_update_storage(&provider, contract, slot, new_val).await?;

        assert_eq!(get_storage_val(&provider, contract, slot).await?, new_val);
        Ok(())
    }

    #[tokio::test]
    async fn test_bal_storage_check() -> Result<()> {
        let config = Config::from_env()?;
        let (provider, _anvil_instance) = spawn_anvil_provider(Some(&config.eth_rpc_endpoint))?;
        let token: H160 = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".parse()?;
        let holder: H160 = "0xDAFEA492D9c6733ae3d56b7Ed1ADB60692c98Bc5".parse().unwrap();
        let slot = utils::u256_to_h256(U256::from(9));

        check_slot_links_to_balanceof(
            &provider, 
            token, 
            slot, 
            holder, 
        ).await
    }


}