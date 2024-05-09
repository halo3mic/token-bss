use crate::common::*;
use alloy::{
    providers::ext::DebugApi,
    transports::TransportErrorKind,
    rpc::types::trace::geth::{
        DefaultFrame, GethDebugTracingOptions, 
        GethDefaultTracingOptions, GethTrace,
        GethDebugTracingCallOptions,
    },
};


pub async fn default_trace_call<P, T, N>(
    provider: &P,
    call_request: TransactionRequest, 
    block: Option<BlockNumberOrTag>, 
    trace_fn: Option<TraceFn>,
) -> Result<DefaultFrame> 
    where 
        P: Provider<T, N>, 
        T: Transport + Clone, 
        N: Network,
{
    let mut tracing_options = GethDebugTracingOptions::default();
    tracing_options.config = GethDefaultTracingOptions::default()
        .with_disable_memory(false)
        .with_enable_memory(true)
        .with_disable_stack(false);
    let trace_call_opt = GethDebugTracingCallOptions::default()
        .with_tracing_options(tracing_options);
    let response = match trace_fn {
        None => provider.debug_trace_call(
            call_request, 
            block.unwrap_or(BlockNumberOrTag::Latest), 
            trace_call_opt,
        ).await,
        Some(trace_fn) => {
            let header = provider.get_block(block.unwrap_or_default().into(), false).await?.unwrap().header;
            trace_fn(
                call_request, 
                header, 
                trace_call_opt,
            ).map_err(|err| TransportErrorKind::custom_str(&err.to_string()))
        }
    }?;

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