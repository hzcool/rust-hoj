use axum::{
    http::Request,
    response::IntoResponse
};
use crate::constants;
use crate::utils::jwt::{self, UserToken};
use axum_extra::middleware::{Next};
pub async fn token_decode<B>(mut req: Request<B>, next: Next<B>) -> impl IntoResponse {
    if let Some(token_header) = req.headers().get(constants::AUTHORIZATION) {
        if let Ok(mut token) = token_header.to_str() {
            if token.starts_with("bearer") || token.starts_with("Bearer") {
                    token = token[6..token.len()].trim();
                    if let Ok(claims) = jwt::decode::<UserToken>(token) {
                        req.extensions_mut().insert(claims);
                    }
            }
        }
    }
    next.run(req).await
}
