use axum::{
    http::Request,
    response::IntoResponse
};
use crate::utils::jwt::UserToken;
use crate::types::error_response::ErrorResponse;
use crate::constants;


use axum_extra::middleware::{Next};

pub async fn auth_admin<B>(req: Request<B>, next: Next<B>) -> impl IntoResponse {
    match req.extensions().get::<UserToken>() {
        Some(token_data) => {
            match token_data.role == constants::SUPER_ADMIN || token_data.role == constants::ADMIN  {
                true => Ok(next.run(req).await),
                _ => Err(ErrorResponse::unauthorized_with_str("没有权限"))
            }
        }
        _ => Err(ErrorResponse::unauthorized_with_str("没有登录")),
    }
}

