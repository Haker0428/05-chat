use axum::http::StatusCode;
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Extension, Json,
};
use tracing::info;

use crate::{AppError, AppState, CreateChat, User};

pub(crate) async fn list_chat_handler(
    Extension(user): Extension<User>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let chat = state.fetch_chats(user.ws_id as _).await?;
    info!("user: {:?}", user);
    Ok((StatusCode::OK, Json(chat)).into_response())
}

pub(crate) async fn create_chat_handler(
    Extension(user): Extension<User>,
    State(state): State<AppState>,
    Json(input): Json<CreateChat>,
) -> Result<impl IntoResponse, AppError> {
    let chat = state.create_chat(input, user.ws_id as _).await?;
    Ok((StatusCode::CREATED, Json(chat)).into_response())
}

pub(crate) async fn get_chat_handler(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<impl IntoResponse, AppError> {
    let chat = state.get_chat_by_id(id).await?;
    match chat {
        Some(chat) => Ok(Json(chat)),
        None => Err(AppError::NotFound(format!("chat id {id}"))),
    }
}

// TODO finish this as a homework
pub(crate) async fn update_chat_handler() -> impl IntoResponse {
    "update chat"
}

// TODO finish this as a homework
pub(crate) async fn delete_chat_handler() -> impl IntoResponse {
    "delete chat"
}
