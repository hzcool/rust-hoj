use axum::{
    http::Request,
    response::IntoResponse
};
use crate::utils::jwt::UserToken;
use crate::types::error_response::ErrorResponse;
use axum_extra::middleware::{Next};

pub async fn auth_login<B>(req: Request<B>, next: Next<B>) -> impl IntoResponse {
    match req.extensions().get::<UserToken>() {
        Some(_) => Ok(next.run(req).await),
        None => Err(ErrorResponse::unauthorized_with_str("未登录")),
    }
}