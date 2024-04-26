use crate::common::*;
use alloy::{
    providers::debug::DebugApi,
    rpc::types::trace::geth::{
        GethDefaultTracingOptions, 
        GethDebugTracingOptions, 
        GethDebugTracerConfig, 
        DefaultFrame, 
        GethTrace,
    },
};


pub async fn default_trace_call(
    provider: &RootProviderHttp,
    call_request: TransactionRequest, 
    block: Option<BlockNumberOrTag>
) -> Result<DefaultFrame> {
    let default_tracing_opt = GethDefaultTracingOptions {
        disable_stack: Some(false),
        disable_memory: Some(false),
        disable_return_data: None,
        enable_return_data: None,
        disable_storage: None,
        enable_memory: None,
        debug: None,
        limit: None,
    };
    let tracing_options = GethDebugTracingOptions {
        config: default_tracing_opt,
        tracer_config: GethDebugTracerConfig::default(),
        tracer: None,
        timeout: None,
    };

    let block = block.unwrap_or(BlockNumberOrTag::Latest);
    let response = provider.debug_trace_call(
        call_request, 
        block.into(), 
        tracing_options,
    ).await?;

    if let GethTrace::Default(trace) = response {
        if trace.failed {
            Err(eyre::eyre!("traceCall failed"))
        } else {
            Ok(trace)
        }
    } else {
        Err(eyre::eyre!("Only default traces supported"))
    }
}