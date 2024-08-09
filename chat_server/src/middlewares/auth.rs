use axum::http::StatusCode;
use axum::{
    extract::{FromRequestParts, Request, State},
    middleware::Next,
    response::{IntoResponse, Response},
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};

use tracing::{info, warn};

use crate::AppState;

pub async fn verriy_token(State(state): State<AppState>, req: Request, next: Next) -> Response {
    let (mut parts, body) = req.into_parts();
    let req =
        match TypedHeader::<Authorization<Bearer>>::from_request_parts(&mut parts, &state).await {
            Ok(TypedHeader(Authorization(bearer))) => {
                let token = bearer.token();
                info!("token {}", token);
                match state.dk.verify(token) {
                    Ok(user) => {
                        let mut req = Request::from_parts(parts, body);
                        req.extensions_mut().insert(user);
                        req
                    }
                    Err(e) => {
                        let msg = format!("verify token failed: {}", e);
                        warn!(msg);
                        return (StatusCode::FORBIDDEN, msg).into_response();
                    }
                }
            }
            Err(e) => {
                let msg = format!("parse Authorization header failed: {}", e);
                warn!(msg);
                return (StatusCode::UNAUTHORIZED, msg).into_response();
            }
        };

    next.run(req).await
}
