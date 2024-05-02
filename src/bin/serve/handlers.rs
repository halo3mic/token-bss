use alloy::primitives::{Address, U256};
use serde::{Serialize, Deserialize};
use std::time::Instant;
use axum::{
    response::{Json, IntoResponse, Response as AxumResponse},
    extract::{Path, State},
    http::StatusCode,
};
use tracing::info;
use super::state::{Chain, AppState};


#[derive(Debug, Serialize)]
pub struct Response {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    msg: Option<SearchResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl From<SearchResponse> for Response {
    fn from(msg: SearchResponse) -> Self {
        Self {
            success: true,
            msg: Some(msg),
            error: None,
        }
    }
}

impl From<&AppError> for Response {
    fn from(err: &AppError) -> Self {
        Self {
            success: false,
            msg: None,
            error: Some(err.0.to_string()),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchResponse {
    token: Address,
    contract: Address,
    slot: U256,
    #[serde(rename = "updateRatio")]
    update_ratio: f64,
    lang: String,
}

#[derive(Debug)]
pub struct AppError(eyre::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> AxumResponse {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            serde_json::to_string(&Response::from(&self)).unwrap(),
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

// todo: split expected and unexpected errors
pub async fn search_handler<T>(
    State(app_state): State<AppState<T>>,
    Path((chain_str, token_str)): Path<(String, String)>
) -> Result<Json<Response>, AppError> 
    where T: Sync + Send + Clone + 'static
{
    let rtime0 = Instant::now();
    let request_id = uuid::Uuid::new_v4().as_u128().to_string();
    info!("{}", serde_json::json!({
        "msg": "new_request",
        "handler": "search_handler",
        "id": request_id,
        "args": serde_json::json!({
            "chain": chain_str,
            "token": token_str,
        }),
    }));
    let res = _search_handler(State(app_state), Path((chain_str, token_str))).await;

    let res_str = match &res {
        Ok(Json(res)) => serde_json::to_string(res)?,
        Err(ref err) => serde_json::to_string(&Response::from(err))?,
    };
    let duration_ms = rtime0.elapsed().as_millis();
    info!("{}", serde_json::json!({
        "msg": "new_response",
        "handler": "search_handler",
        "id": request_id,
        "response": res_str,
        "duration": duration_ms,
    }));
    res
}

async fn _search_handler<T>(
    State(app_state): State<AppState<T>>,
    Path((chain_str, token_str)): Path<(String, String)>
) -> Result<Json<Response>, AppError> 
    where T: Sync + Send + Clone + 'static
{
    let chain = chain_str.parse::<Chain>()?;
    let token: Address = token_str.parse().map_err(|_| eyre::eyre!("Invalid token"))?;

    if let Some(db_conn) = &app_state.db_connection {
        let mut db_conn = db_conn.lock().unwrap();
        let entry = db_conn.get_entry(&token, &chain)?;
        if let Some(entry) = entry {
            return Ok(Json(Response::from(entry)));
        }
    }

    let endpoint = &app_state.providers
        .get(&chain)
        .ok_or_else(|| eyre::eyre!(format!("Can't find provider for chain: {chain:?}")))?
        .endpoint;
    // todo: someone could spam tokens that don't exist or take a long time to process (set a limit + store err in db)
    let response = erc20_topup::find_slot(&endpoint, token, None).await?;

    let response = SearchResponse {
        token: token,
        contract: response.0,
        slot: response.1.into(),
        update_ratio: response.2,
        lang: response.3,
    };

    if let Some(db_conn) = app_state.db_connection {
        let mut db_conn = db_conn.lock().unwrap();
        db_conn.store_entry(&token, &chain, &response)?; // todo this err should not be propagated to the user
    }

    Ok(Json(response.into()))
}