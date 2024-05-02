use alloy::primitives::{Address, U256};
use serde::{Serialize, Deserialize};
use std::time::Instant;
use tokio::time::{timeout, Duration};
use axum::{
    response::{Json, IntoResponse, Response as AxumResponse},
    extract::{Path, State},
    http::StatusCode,
};
use tracing::{info, error};
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

impl From<&UserError> for Response {
    fn from(err: &UserError) -> Self {
        Self {
            success: false,
            msg: None,
            error: Some(format!("{err:?}")),
        }
    }
}

impl From<&eyre::Report> for Response {
    fn from(err: &eyre::Report) -> Self {
        Self {
            success: false,
            msg: None,
            error: Some(format!("{err:#}")),
        }
    }
}

impl From<&AppError> for Response {
    fn from(err: &AppError) -> Self {
        match err {
            AppError::UserError(err) => Self::from(err),
            AppError::InternalError(err) => Self::from(err),
        }
    }

}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SearchResponseWrapper {
    Found(SearchResponse),
    NotFound,
}

impl From<SearchResponse> for SearchResponseWrapper {
    fn from(msg: SearchResponse) -> Self {
        Self::Found(msg)
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
pub enum AppError {
    UserError(UserError),
    InternalError(eyre::Error),
}

#[derive(Debug)]
pub enum UserError {
    InvalidToken,
    ChainNotFound,
    ProviderNotFound,
    Timeout,
    SlotNotFound,
}

impl IntoResponse for AppError {
    fn into_response(self) -> AxumResponse {
        match self {
            AppError::UserError(err) => match err {
                UserError::InvalidToken | UserError::ChainNotFound | UserError::ProviderNotFound => (
                    StatusCode::BAD_REQUEST,
                    serde_json::to_string(&Response::from(&err)).unwrap(),
                )
                    .into_response(),
                UserError::Timeout => (
                    StatusCode::GATEWAY_TIMEOUT,
                    serde_json::to_string(&Response::from(&err)).unwrap(),
                )
                    .into_response(),
                UserError::SlotNotFound => (
                    StatusCode::NOT_FOUND,
                    serde_json::to_string(&Response::from(&err)).unwrap(),
                )
                    .into_response(),
            },
            AppError::InternalError(err) => {
                error!("{:#}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    serde_json::to_string(&Response::from(&eyre::eyre!("InternalError"))).unwrap(),
                ).into_response()
            }
        }
    }
}

impl<E> From<E> for AppError
where
    E: Into<eyre::Error>,
{
    fn from(err: E) -> Self {
        Self::InternalError(err.into())
    }
}

#[derive(Debug, Serialize)]
enum InfoSource {
    Provider,
    Database,
}

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
    let tm_out = app_state.timeout_ms;
    let fut = _search_handler(State(app_state), Path((chain_str, token_str)));
    let res = match timeout(Duration::from_millis(tm_out), fut).await {
        Ok(res) => res,
        Err(_) => Err(AppError::UserError(UserError::Timeout)),
    };

    let (res_str, source) = match &res {
        Ok((Json(res), source)) => (serde_json::to_string(res)?, Some(source)),
        Err(ref err) => (serde_json::to_string(&Response::from(err))?, None),
    };
    let duration_ms = rtime0.elapsed().as_millis();
    let source = source.map(|s| format!("{s:?}")).unwrap_or("null".to_string());
    info!("{}", serde_json::json!({
        "msg": "new_response",
        "handler": "search_handler",
        "id": request_id,
        "response": res_str,
        "duration": duration_ms,
        "source": source,
    }));
    res.map(|(res, _)| res)
}

async fn _search_handler<T>(
    State(app_state): State<AppState<T>>,
    Path((chain_str, token_str)): Path<(String, String)>
) -> Result<(Json<Response>, InfoSource), AppError> 
    where T: Sync + Send + Clone + 'static
{
    let chain = chain_str.parse::<Chain>()
        .map_err(|_| AppError::UserError(UserError::ChainNotFound))?;
    let token: Address = token_str.parse()
        .map_err(|_| AppError::UserError(UserError::InvalidToken))?;

    if let Some(db_conn) = &app_state.db_connection {
        let mut db_conn = db_conn.lock().unwrap();
        let response = db_conn.get_search_response(&token, &chain)?;
        if let Some(response) = response {
            return match response {
                SearchResponseWrapper::NotFound => Err(AppError::UserError(UserError::SlotNotFound)),
                SearchResponseWrapper::Found(entry) => Ok((Json(Response::from(entry)), InfoSource::Database)),
            };
        }
    }

    let endpoint = &app_state.providers
        .get(&chain)
        .ok_or(AppError::UserError(UserError::ProviderNotFound))?
        .endpoint;
    let response = erc20_topup::find_slot(&endpoint, token, None).await
        .map_err(|err| {
            if err.to_string().contains("No valid slots found") {
                if let Some(db_conn) = &app_state.db_connection {
                    let mut db_conn = db_conn.lock().unwrap();
                    let response = SearchResponseWrapper::NotFound;
                    db_conn.store_search_response(&token, &chain, &response).unwrap(); // todo dont unwrap!
                }
                AppError::UserError(UserError::SlotNotFound)
            } else {
                AppError::InternalError(err)
            }
        })?;

    let response = SearchResponse {
        token: token,
        contract: response.0,
        slot: response.1.into(),
        update_ratio: response.2,
        lang: response.3,
    };

    if let Some(db_conn) = app_state.db_connection {
        let mut db_conn = db_conn.lock().unwrap();
        let response = response.clone().into();
        db_conn.store_search_response(&token, &chain, &response)?;
    }

    Ok((Json(response.into()), InfoSource::Provider))
}