use alloy::transports::Transport;
use alloy::providers::Provider;
use axum::{Router, routing::get};
use tracing::info;
use eyre::Result;

use crate::handlers::search_handler;
use crate::state::AppState;


pub async fn run<P, T, H>(addr: &str, state: AppState<P, T, H>) -> Result<()> 
    where P: Provider<T> + Clone + 'static, T: Transport + Clone, H: Sync + Send + Clone + 'static
{
    let app = Router::new()
        .route("/:chain/:token", get(search_handler))
        .with_state(state);

    info!("Server running on: {addr}");

    axum::Server::bind(&addr.parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}
