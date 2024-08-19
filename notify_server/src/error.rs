use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("io found: {0}")]
    IoError(#[from] std::io::Error),

    #[error("jwt error: {0}")]
    JwtError(#[from] jwt_simple::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let status = match &self {
            Self::JwtError(_) => StatusCode::FORBIDDEN,
            Self::IoError(_) => StatusCode::CONFLICT,
        };

        (status, Json(serde_json::json!({"error": self.to_string()}))).into_response()
    }
}
