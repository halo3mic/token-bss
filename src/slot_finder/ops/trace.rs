use ethers::prelude::*;
use eyre::Result;
use alloy::rpc::types::trace::geth::{
    GethDebugTracingCallOptions,
    GethDefaultTracingOptions,
    GethDebugTracingOptions,
    GethDebugTracerConfig,
};
use ethers::types::{GethTrace, GethTraceFrame, DefaultFrame};


pub async fn default_trace_call(
    provider: &Provider<Http>, 
    call_request: TransactionRequest, 
    block: Option<BlockId>
) -> Result<DefaultFrame> {
    let config = GethDefaultTracingOptions {
        disable_storage: Some(false),
        disable_stack: Some(false),
        disable_memory: Some(false),
        disable_return_data: Some(false),
        enable_memory: Some(true),
        enable_return_data: Some(true),
        debug: None,
        limit: None,
    };
    let tracing_options = GethDebugTracingOptions {
        tracer_config: GethDebugTracerConfig::default(),
        config: config,
        tracer: None,
        timeout: None,
    };
    let call_options = GethDebugTracingCallOptions {
        tracing_options: tracing_options,
        state_overrides: None,
        block_overrides: None,
    };
    // ! Anvil supports alloy's tracer API, ethers-rs's is different
    let response = provider.request(
        "debug_traceCall", 
        vec![
            serde_json::json!(call_request), 
            serde_json::json!(block),
            serde_json::json!(call_options)
        ]
    ).await?;
    let parsedResponse = serde_json::from_value::<GethTrace>(response)?;

    match parsedResponse {
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
