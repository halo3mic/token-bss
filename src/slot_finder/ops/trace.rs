use ethers::prelude::*;
use ethers::types::{
    GethDebugTracingCallOptions, 
    GethDebugTracingOptions,
    GethTraceFrame, 
    GethTrace, 
};
use eyre::Result;


pub async fn default_trace_call(
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
