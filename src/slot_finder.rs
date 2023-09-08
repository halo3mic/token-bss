// todo: check which tokens are supported
// todo: change token balance
// todo: usage from cli

use ethers::prelude::*;
use ethers::providers::{Provider, Http};
use ethers::types::{GethDebugTracingCallOptions, GethTrace, GethTraceFrame, StructLog, GethDebugTracingOptions};
use ethers::utils::hex::FromHex;
use eyre::Result;
use std::collections::HashMap;


pub async fn find_slot_from_call(provider: Provider<Http>) -> Result<()> {
    let fb_builder: H160 = "0xDAFEA492D9c6733ae3d56b7Ed1ADB60692c98Bc5".parse().unwrap();
    let token: H160 = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".parse().unwrap();
    
    let data = format!("0x70a08231000000000000000000000000{}", format!("{:?}", fb_builder)[2..].to_string());

    let tx_request = TransactionRequest::new()
        .from(fb_builder)
        .to(token)
        .data(Bytes::from_hex(data)?);
    let response = default_trace_call(provider, tx_request, None).await?;
    if response.failed {
        return Err(eyre::eyre!("Call failed"));
    }
    let return_val = H256::from_slice(&response.return_value.to_vec());
    let results = find_slot(response.struct_logs, fb_builder, token, &return_val)?;

    println!("Results: {:?}", results);
    // todo: for every result that it is correct slot -> changes balanceOf


    Ok(())
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
    let response = provider.debug_trace_call(call_request, block, call_options).await?;
    match response {
        GethTrace::Known(GethTraceFrame::Default(trace)) => {
            Ok(trace)
        },
        _ => {
            Err(eyre::eyre!("Only known default traces supported"))
        }
    }
}

fn find_slot(
    struct_logs: Vec<StructLog>, 
    from_address: H160, 
    to_address: H160, 
    return_val: &H256
) -> Result<Vec<(H160, U256)>> {
    let mut depth_to_address = HashMap::new();
    depth_to_address.insert(1, to_address);
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
                if sload_address == from_address {
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

    #[tokio::test]
    async fn test_trace_call() {
        dotenv::dotenv().ok();
        // get ETH_RPC environment variable
        let rpc_url = std::env::var("ETH_RPC").expect("ETH_RPC environment variable not set"); // todo: move to config
        // setup anvil 
        use ethers::utils::Anvil;

        // todo: move to utils
        let anvil_fork = Anvil::new().fork(rpc_url).spawn();    
        let provider = Provider::<Http>::try_from(anvil_fork.endpoint()).unwrap();


        find_slot_from_call(provider).await.unwrap();


    }


}