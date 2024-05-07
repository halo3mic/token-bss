use crate::common::*;
use alloy::{
    providers::ext::DebugApi,
    rpc::types::trace::geth::{
        DefaultFrame, GethDebugTracingOptions, 
        GethDefaultTracingOptions, GethTrace,
        GethDebugTracingCallOptions,
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
    let trace_call_opt = GethDebugTracingCallOptions::default()
        .with_tracing_options(GethDebugTracingOptions::default());

    let response = provider.debug_trace_call(
        call_request, 
        block.unwrap_or(BlockNumberOrTag::Latest), 
        trace_call_opt,
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