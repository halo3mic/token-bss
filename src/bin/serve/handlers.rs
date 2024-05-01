use alloy::primitives::{Address, U256};
use serde::{Serialize, Deserialize};
use std::time::Instant;
use axum::{
    response::{Json, IntoResponse, Response as AxumResponse},
    http::StatusCode,
    extract::{Path, State},
};
use super::state::{Chain, AppState};


#[derive(Debug, Serialize)]
struct Response {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    msg: Option<SearchResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResponse {
    token: Address,
    contract: Address,
    slot: U256,
    #[serde(rename = "updateRatio")]
    update_ratio: f64,
    lang: String,
}

pub struct AppError(eyre::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> AxumResponse {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            serde_json::to_string(&Response {
                success: false,
                msg: None,
                error: Some(self.0.to_string()),
            }).unwrap(),
        )
            .into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<eyre::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

pub async fn search_handler<T>(
    State(app_state): State<AppState<T>>,
    Path((chain_str, token_str)): Path<(String, String)>
) -> Result<Json<SearchResponse>, AppError> 
    where T: Sync + Send + Clone + 'static
{
    // todo: logging + clean
    let chain = chain_str.parse::<Chain>()?;
    let token: Address = token_str.parse().map_err(|_| eyre::eyre!("Invalid token"))?;

    println!("Searching for token: {token:?} on chain {chain:?}");
    let now = Instant::now();

    let endpoint = &app_state.providers
        .get(&chain)
        .ok_or_else(|| eyre::eyre!(format!("Can't find provider for chain: {chain:?}")))?
        .endpoint;
    
    let response = erc20_topup::find_slot(&endpoint, token, None).await?;

    let t2 = now.elapsed();
    println!("{:?}", response);
    let response = SearchResponse {
        token: token,
        contract: response.0,
        slot: response.1.into(),
        update_ratio: response.2,
        lang: response.3,
    };
    println!("{:?}", response);
    println!("Time taken: {:?}", t2);

    Ok(Json(response))
}
