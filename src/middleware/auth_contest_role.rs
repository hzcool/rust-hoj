use axum::{
    http::Request,
    extract::Path,
};
use axum::extract::{FromRequest, RequestParts};
use axum::response::IntoResponse;
use crate::types::error_response::ErrorResponse;
use crate::dao::contest_dao as cd;
use crate::utils::jwt::UserToken;

use axum_extra::middleware::{Next};




pub async fn auth_contest_role<B: std::marker::Send>(req: Request<B>, next: Next<B>) -> impl IntoResponse {
    let mut req_parts = RequestParts::new(req);
    let mp: Path::<std::collections::HashMap<String, String>> = Path::from_request(&mut req_parts).await.unwrap();
    let cid: i64 = mp.0.get("id").unwrap_or(&"0".to_string()).parse().unwrap_or(0);
    if cid == 0 {
        return Err(ErrorResponse::not_found_with_str("Not Found"))
    }
    match req_parts.extensions().unwrap().get::<UserToken>() {
        Some(token_data) =>
            match cd::allowed_enter_of_user(cid, token_data.id).await {
                Ok(_) => {
                    let req = req_parts.try_into_request().unwrap();
                    Ok(next.run(req).await)
                }
                Err(_) => Err(ErrorResponse::unauthorized_with_str("未注册")),
            },
        None => Err(ErrorResponse::unauthorized_with_str("未登录")),
    }
}