use serde::{Deserialize, Serialize};
use serde_json::Value as Json;
use axum::{
    Json as AxumJson,
    response::{
        IntoResponse,
        Response as AxumResponse
    }
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Response<T: Serialize> {
    pub msg: String,
    pub data: T,
}

impl<T> Response<T>
    where
        T: Serialize,
{
    pub fn new(message: &str, data: T) -> Self {
        Self {
            msg: message.to_string(),
            data,
        }
    }
    pub fn from(data: T) -> Self {
        Self {
            msg: "ok".to_string(),
            data,
        }
    }
}

impl Response<Json> {
    pub fn from_msg(msg: &str) -> Self {
        Self {
            msg: msg.to_string(),
            data: Json::Null,
        }
    }
}

impl<T: Serialize> IntoResponse for Response<T> {
    fn into_response(self) -> AxumResponse {
        AxumJson::from(self).into_response()
    }
}