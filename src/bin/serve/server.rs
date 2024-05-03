use axum::{Router, routing::get};
use tracing::info;
use eyre::Result;

use crate::handlers::search_handler;
use crate::state::AppState;


pub async fn run<T>(addr: &str, state: AppState<T>) -> Result<()> 
    where T: Sync + Send + Clone + 'static
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
