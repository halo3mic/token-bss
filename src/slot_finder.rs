// todo: check which tokens are supported
// todo: change token balance
// todo: usage from cli (use bin)
// todo: export to csv (bin)
// ? All slot refs to H256 (instead of U256)?

use ethers::prelude::*;
use ethers::providers::{Provider, Http};
use ethers::types::{GethDebugTracingCallOptions, GethTrace, GethTraceFrame, StructLog, GethDebugTracingOptions};
use ethers::utils::hex::FromHex;
use ethers::abi::{Tokenizable, Token};
use eyre::Result;
use std::collections::{HashMap, HashSet};
use ethers::types::transaction::eip2718::TypedTransaction;
use super::utils;
use rand;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    Solidity,
    Vyper,
}

const balanceof_4byte: &str = "0x70a08231";

pub async fn find_balance_slots_and_update_ratio(
    provider: &Provider<Http>,
    holder: H160, 
    token: H160,
) -> Result<(H160, U256, f64)> {
    let slots = find_balance_slots(provider, holder, token).await?;
    for (contract, slot, lang) in slots {
        let slot_h256 = utils::u256_to_h256(slot);
        if let Ok(update_ratio) = 
            slot_update_to_bal_ratio(
                &provider, 
                token, 
                contract, 
                slot_h256, 
                holder, 
                lang
            ).await 
        {
            return Ok((contract, slot, update_ratio));
        }
    }
    Err(eyre::eyre!("No valid slots found"))
}

pub async fn find_balance_slots(
    provider: &Provider<Http>,
    holder: H160, 
    token: H160,
) -> Result<Vec<(H160, U256, Language)>> {
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
async fn slot_update_to_bal_ratio(
    provider: &Provider<Http>, 
    token: H160,
    storage_contract: H160, // Contract where slot is located
    slot: H256,
    holder: H160,
    lang: Language,
) -> Result<f64> {
    let map_loc = match lang {
        Language::Solidity => solidity_mapping_loc(&slot, &H256::from(holder)),
        Language::Vyper => vyper_mapping_loc(&slot, &H256::from(holder)),
    };
    println!("Map loc: {:#?}", map_loc);
    let old_val = get_storage_val(provider, storage_contract, map_loc).await?;
    println!("Old val: {:#?}", old_val);
    let new_val_u64 = rand::random::<u128>();
    let new_val = utils::u256_to_h256(U256::from(new_val_u64));
    println!("New val: {:#?}", new_val);
    anvil_update_storage(provider, storage_contract, map_loc, new_val).await?;
    println!("Updated storage");
    let mut call_request = TypedTransaction::Legacy(balanceof_call_req(holder, token)?);
    call_request.set_gas(100_000); // ! Necessary to set gas otherwise changing the wrong storage could incur a lot of processing eg. 0xf25c91c87e0b1fd9b4064af0f427157aab0193a7(Ethereum)
    println!("Call request: {:#?}", call_request);
    let balance = provider.call(&call_request, None).await?;
    println!("Balance: {:#?}", balance);
    let balance = utils::bytes_to_h256(balance);
    anvil_update_storage(provider, storage_contract, map_loc, old_val).await?; // Change the storage value back to the original
    if balance == old_val {
        return Err(eyre::eyre!("BalanceOf reflects old storage"));
    }
    let ur_bn = utils::h256_to_u256(balance) * U256::from(10_000) / U256::from(new_val_u64);
    let update_ratio = if ur_bn <= U256::max_value() / U256::from(2) {
        ur_bn.as_u128()
    } else {
        u128::max_value()
    } as f64 / 10_000.;
    Ok(update_ratio)
}

pub fn solidity_mapping_loc(storage_index: &H256, key: &H256) -> H256 {
    mapping_loc(key.into_token(), storage_index.into_token())
}

pub fn vyper_mapping_loc(storage_index: &H256, key: &H256) -> H256 {
    mapping_loc(storage_index.into_token(), key.into_token())
}

pub fn mapping_loc(token_0: Token, token_1: Token) -> H256 {
    let hash_input = ethers::abi::encode(&[ token_0, token_1 ]);
    ethers::utils::keccak256(hash_input).into()
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
    provider: &Provider<Http>, 
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
) -> Result<Vec<(H160, U256, Language)>> {
    let mut depth_to_address = HashMap::new();
    depth_to_address.insert(1, token);
    let mut results = HashSet::new();
    let mut hashed_vals = HashMap::new();
    
    for log in struct_logs {
        let depth = log.depth;
        
        if log.op == "SLOAD" {
            // handle_sload_op
            println!("{:#?}", log.clone());
            if log.memory.as_ref().map(|m|m.len() < 2).unwrap_or(true) || log.stack.as_ref().is_none() || log.storage.as_ref().is_none() {
                continue;
            }
            
            for slot_idx in log.storage.as_ref().unwrap().keys() {
                // ! Refrain from checking balanceOf return val matches storage val as it is not always the case
                if let Some((hashed_val_0, hashed_val_1)) = hashed_vals.get(slot_idx) {
                    let (slot, lang) = 
                        if &H256::from(holder) == hashed_val_0 {
                            (*hashed_val_1, Language::Solidity)
                        } else if &H256::from(holder) == hashed_val_1 {
                            (*hashed_val_0, Language::Vyper)
                        } else {
                            continue;
                        };
                    let contract = depth_to_address.get(&depth).unwrap();
                    results.insert((*contract, utils::h256_to_u256(slot), lang));
                }
            }
        } else if log.op == "STATICCALL" || log.op == "CALL" {
            // handle_staticcall_op
            println!("staticcall/call: {:#?}", log.clone());
            let stack = log.stack.unwrap();
            let address = utils::u256_to_h160(stack[stack.len()-2]);
            depth_to_address.insert(depth + 1, address);
        } else if log.op == "DELEGATECALL" {
            // handle_delegatecall_op
            println!("delegatecall: {:#?}", log.clone());
            let prev_address = *depth_to_address.get(&depth).unwrap();
            depth_to_address.insert(depth + 1, prev_address);
        } else if log.op == "SHA3" {
            // handle_sha3_op
            let memory = hex::decode(log.memory.as_ref()
                .expect("SHA3 op should have memory content")
                .join("")
            )?;
            let stack = log.stack.as_ref()
                .expect("SHA3 op should have stack content");
            let mem_offset = stack[stack.len()-1].as_usize();
            let mem_length = stack[stack.len()-2].as_usize();
            if mem_length == 64 { // Only concerned about storage mappings
                let hashed_val = memory[mem_offset..mem_offset+mem_length].to_vec();
                let hash = H256(ethers::utils::keccak256(&hashed_val));
                let hashed_val_0: [u8; 32] = hashed_val[0..32].to_vec().try_into().unwrap();
                let hashed_val_1: [u8; 32] = hashed_val[32..64].to_vec().try_into().unwrap();
                hashed_vals.insert(hash, (H256(hashed_val_0), H256(hashed_val_1)));
            }
            println!("sha3: {:#?}", log.clone());
        }
    }

    Ok(results.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::utils;

    #[tokio::test]
    async fn test_slot_finding() -> Result<()> {
        let config = Config::from_env()?;
        let (provider, _anvil_instance) = utils::spawn_anvil_provider(Some(&config.eth_rpc_endpoint))?;

        let token: H160 = "0xC011a73ee8576Fb46F5E1c5751cA3B9Fe0af2a6F".parse().unwrap();
        let holder: H160 = "0x0Ff31c2544B2288C6544aAD39dEF9Ab2472404F8".parse().unwrap();

        let result = find_balance_slots(&provider, holder, token).await?;
        // println!("{:#?}", result);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "0x5b1b5fea1b99d83ad479df0c222f0492385381dd".parse::<H160>().unwrap());
        assert_eq!(result[0].1, U256::from(3));

        Ok(())
    }

    #[tokio::test]
    async fn test_update_slot_val() -> Result<()> {
        let (provider, _anvil_instance) = utils::spawn_anvil_provider(None)?;

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
        let (provider, _anvil_instance) = utils::spawn_anvil_provider(Some(&config.eth_rpc_endpoint))?;
        let token: H160 = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".parse()?;
        let holder: H160 = "0xDAFEA492D9c6733ae3d56b7Ed1ADB60692c98Bc5".parse().unwrap();
        let slot = utils::u256_to_h256(U256::from(9));
        let lang = Language::Solidity;

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
    async fn test_bal_storage_check_b() -> Result<()> {
        let config = Config::from_env()?;
        let (provider, _anvil_instance) = utils::spawn_anvil_provider(Some(&config.eth_rpc_endpoint))?;
        let token: H160 = "0xfE18be6b3Bd88A2D2A7f928d00292E7a9963CfC6".parse()?;
        let holder: H160 = "0xDAFEA492D9c6733ae3d56b7Ed1ADB60692c98Bc5".parse().unwrap();
        let result = find_balance_slots(&provider, holder, token).await?;

        assert_eq!(result.len(), 1);
        let (contract, slot, lang) = result[0];
        let update_ratio = slot_update_to_bal_ratio(
            &provider, 
            token,
            contract, 
            utils::u256_to_h256(slot), 
            holder,
            lang,
        ).await?;
        assert_eq!(update_ratio, 1.0);

        Ok(())
    }

    #[tokio::test]
    async fn test_bal_storage_check_c() -> Result<()> {
        let config = Config::from_env()?;
        let (provider, _anvil_instance) = utils::spawn_anvil_provider(Some(&config.eth_rpc_endpoint))?;
        let token: H160 = "0xb8b295df2cd735b15BE5Eb419517Aa626fc43cD5".parse()?;
        let holder: H160 = "0xDAFEA492D9c6733ae3d56b7Ed1ADB60692c98Bc5".parse().unwrap();
        let result = find_balance_slots(&provider, holder, token).await?;

        assert_eq!(result.len(), 1);
        let (contract, slot, lang) = result[0];
        let ratio = slot_update_to_bal_ratio(
            &provider, 
            token,
            contract, 
            utils::u256_to_h256(slot), 
            holder, 
            lang,
        ).await?;
        println!("Ratio: {:#?}", ratio);

        Ok(())
    }

    #[tokio::test]
    async fn test_bal_storage_check_d() -> Result<()> {
        let config = Config::from_env()?;
        let (provider, _anvil_instance) = utils::spawn_anvil_provider(Some(&config.eth_rpc_endpoint))?;
        let token: H160 = "0x6c3f90f043a72fa612cbac8115ee7e52bde6e490".parse()?;
        let holder: H160 = "0xDAFEA492D9c6733ae3d56b7Ed1ADB60692c98Bc5".parse().unwrap();
        let result = find_balance_slots(&provider, holder, token).await?;

        println!("Result: {:#?}", result);
        let (contract, slot, lang) = result[0];

        let ratio = slot_update_to_bal_ratio(
            &provider, 
            token,
            contract, 
            utils::u256_to_h256(slot), 
            holder,
            lang, 
        ).await?;


        println!("Ratio: {:#?}", ratio);

        Ok(())
    }

    #[tokio::test]
    async fn test_bal_storage_check_e() -> Result<()> {
        let config = Config::from_env()?;
        let (provider, _anvil_instance) = utils::spawn_anvil_provider(Some(&config.eth_rpc_endpoint))?;
        let token: H160 = "0xf25c91c87e0b1fd9b4064af0f427157aab0193a7".parse()?;
        let holder: H160 = "0xDAFEA492D9c6733ae3d56b7Ed1ADB60692c98Bc5".parse().unwrap();
        let result = find_balance_slots(&provider, holder, token).await?;

        println!("Result: {:#?}", result);
        for (contract, slot, lang) in result {
            if let Ok(ratio) = slot_update_to_bal_ratio(
                &provider, 
                token,
                contract, 
                utils::u256_to_h256(slot), 
                holder,
                lang, 
            ).await {
                println!("Ratio: {:#?}", ratio);
            }
        }

        Ok(())
    }

}