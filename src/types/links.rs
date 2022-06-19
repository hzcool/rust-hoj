use super::error_response::ErrorResponse;
use serde_json::{Map, Value as Json};
pub type JsonMap = Map<String, Json>;
pub type ResponseResult = Result<axum::response::Response, ErrorResponse>;

