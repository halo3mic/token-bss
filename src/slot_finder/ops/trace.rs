use crate::common::*;
use alloy::{
    providers::debug::DebugApi,
    rpc::types::trace::geth::{
        GethDefaultTracingOptions, 
        GethDebugTracingOptions, 
        DefaultFrame, 
        GethTrace,
    },
};


pub async fn default_trace_call(
    provider: &RootProviderHttp,
    call_request: TransactionRequest, 
    block: Option<BlockNumberOrTag>
) -> Result<DefaultFrame> {
    let mut tracing_options = GethDebugTracingOptions::default();
    tracing_options.config = GethDefaultTracingOptions::default()
        .with_disable_memory(false)
        .with_enable_memory(true)
        .with_disable_stack(false);

    let response = provider.debug_trace_call(
        call_request, 
        block.unwrap_or(BlockNumberOrTag::Latest), 
        tracing_options,
    ).await?;

    match response {
        GethTrace::Default(trace) if trace.failed => {
            Err(eyre::eyre!("traceCall failed"))
        },
        GethTrace::Default(trace) if !trace.failed => {
            Ok(trace)
        },
        _ => Err(eyre::eyre!("Only default traces supported")),
    }
}