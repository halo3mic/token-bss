use alloy::{
    providers::{debug::DebugApi, RootProvider}, rpc::types::{
        eth::{BlockNumberOrTag, TransactionRequest}, 
        trace::geth::{
            DefaultFrame, GethDebugTracerConfig, GethDebugTracingOptions, GethDefaultTracingOptions, GethTrace
        },
    }, transports::http::Http
};
use reqwest::Client;
use eyre::Result;


pub async fn default_trace_call(
    provider: &RootProvider<Http<Client>>,
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
    // BlockNumberOrTag
    // BlockId
    let block = block.unwrap_or(BlockNumberOrTag::Latest);
    let response = provider.debug_trace_call(
        call_request, 
        block.into(), 
        tracing_options,
    ).await?;

    match response {
        GethTrace::Default(trace) => {
            if trace.failed {
                Err(eyre::eyre!("traceCall failed"))
            } else {
                Ok(trace)
            }
        },
        _ => {
            Err(eyre::eyre!("Only default traces supported"))
        }
    }
}