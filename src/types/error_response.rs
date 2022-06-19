use super::error::{Error, ErrorType};
use super::response::Response;
use axum::{
    http::StatusCode,
    response::{
        Response as AxumResponse,
        IntoResponse
    }
};

use std::convert::From;
use std::error::Error as StdError;
use std::fmt::{Debug, Display, Formatter};

#[derive(Debug)]
pub struct ErrorResponse {
    status: StatusCode,
    info: String,
}

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> AxumResponse {
        let resp = Response::from_msg(self.info.as_str()).into_response();
        AxumResponse::builder()
            .status(self.status)
            .body(resp.into_body())
            .unwrap()
    }
}

impl Display for ErrorResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.info)
    }
}

impl From<Box<dyn StdError>> for ErrorResponse {
    fn from(err: Box<dyn StdError>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            info: format!("{}", err),
        }
    }
}

impl From<Error> for ErrorResponse {
    fn from(e: Error) -> Self {
        match e.error_type {
            ErrorType::NoneError => ErrorResponse::not_found_with_str(e.error.as_str()),
            ErrorType::SystemError => ErrorResponse::server_error_with_str(e.error.as_str()),
            ErrorType::Unauthorized => ErrorResponse::unauthorized_with_str(e.error.as_str()),
            ErrorType::Forbidden => ErrorResponse::forbidden_with_str(e.error.as_str()),
            ErrorType::BadRequest => ErrorResponse::bad_request_with_str(e.error.as_str()),
        }
    }
}

impl From<anyhow::Error> for ErrorResponse {
    fn from(e: anyhow::Error) -> Self {
        ErrorResponse::server_error_with_str(format!("{}", e).as_str())
    }
}

impl ErrorResponse {
    pub fn from(status: StatusCode, info: String) -> Self {
        Self { status, info }
    }
    pub fn not_found(err: Box<dyn StdError>) -> Self {
        ErrorResponse::from(StatusCode::NOT_FOUND, format!("{}", err))
    }
    pub fn not_found_with_str(err: &str) -> Self {
        ErrorResponse::from(StatusCode::NOT_FOUND, format!("{}", err))
    }

    pub fn server_error(err: Box<dyn StdError>) -> Self {
        ErrorResponse::from(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", err))
    }
    pub fn server_error_with_str(err: &str) -> Self {
        ErrorResponse::from(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", err))
    }
    pub fn server_error_default() -> Self {
        ErrorResponse::from(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("服务器错误，请求失败"),
        )
    }
    pub fn forbidden(err: Box<dyn StdError>) -> Self {
        ErrorResponse::from(StatusCode::FORBIDDEN, format!("{}", err))
    }
    pub fn forbidden_with_str(err: &str) -> Self {
        ErrorResponse::from(StatusCode::FORBIDDEN, format!("{}", err))
    }
    pub fn unauthorized(err: Box<dyn StdError>) -> Self {
        ErrorResponse::from(StatusCode::UNAUTHORIZED, format!("{}", err))
    }
    pub fn unauthorized_default() -> Self {
        ErrorResponse::from(StatusCode::UNAUTHORIZED, format!("没有权限"))
    }
    pub fn unauthorized_with_str(err: &str) -> Self {
        ErrorResponse::from(StatusCode::FORBIDDEN, format!("{}", err))
    }
    pub fn bad_request(err: Box<dyn StdError>) -> Self {
        ErrorResponse::from(StatusCode::BAD_REQUEST, format!("{}", err))
    }
    pub fn bad_request_with_str(err: &str) -> Self {
        ErrorResponse::from(StatusCode::BAD_REQUEST, format!("{}", err))
    }
}