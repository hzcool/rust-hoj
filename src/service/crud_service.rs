use crate::dao::{contest_dao as cd, csubmisssion_dao as csd, problem_dao as pd, submission_dao as sd, tsubmission_dao as tsd, user_dao as ud, post_dao, crud};
use crate::utils::jwt;
use crate::constants;
use crate::types::{
    error_response::ErrorResponse,
    links::{JsonMap, ResponseResult},
    response::Response,
};

use serde::Deserialize;
use std::collections::HashMap;
use axum::{
    response::IntoResponse,
    extract::{Path, Query, Json as AxumJson, Extension}
};
use serde_json::json;
use crate::model::post::Post;
use crate::model::problem::Problem;
use crate::model::submission::Submission;

pub async fn last_index(Path(is_open): Path<bool>) -> ResponseResult {
    Ok(Response::from(pd::last_index(is_open).await?).into_response())
}

pub async fn get_problem_id(Query(qry): Query<HashMap<String, String>>) -> ResponseResult {
    let index = match qry.get("index") {
        Some(s) => s.as_str(),
        _ => return Err(ErrorResponse::bad_request_with_str("请求参数错误")),
    };
    Ok(Response::from(pd::get_problem_id(index).await?).into_response())
}

#[derive(Deserialize)]
pub struct FindBody {
    pub filter: Option<JsonMap>,
    pub offset: Option<i32>,
    pub limit: Option<i32>,
    pub desc: Option<bool>, //是否倒序查找
}

pub async fn find(
    Path(table_name): Path<String>,
    AxumJson(mut body): AxumJson<FindBody>,
    token_data: Option<Extension<jwt::UserToken>>,
) -> ResponseResult {
    match table_name.as_str() {
        constants::USER_TABLE_NAME => Ok(Response::from(
            ud::find(body.filter, body.desc, body.limit, body.offset).await?,
        )
            .into_response()),
        constants::PROBLEM_TABLE_NAME => {
            // 必须包含is_open信息
            let f = match &mut body.filter {
                None => return Err(ErrorResponse::bad_request_with_str("参数错误")),
                Some(x) => x,
            };
            let is_open = match f.remove("is_open") {
                None => return Err(ErrorResponse::bad_request_with_str("参数错误")),
                Some(v) => v.as_bool().unwrap_or(true),
            };
            if !is_open && (token_data.is_none() || !token_data.unwrap().is_admin()) {
                return Err(ErrorResponse::unauthorized_default());
            }
            Ok(Response::from(
                pd::find(is_open, body.filter, body.desc, body.limit, body.offset).await?,
            )
                .into_response())
        }
        constants::SUBMISSION_TABLE_NAME => {
            Ok(
                Response::from(sd::find(body.filter, body.desc, body.limit, body.offset).await?)
                    .into_response(),
            )
        }
        constants::CONTEST_TABLE_NAME => Ok(Response::from(
            cd::find(body.filter, body.desc, body.limit, body.offset).await?,
        )
            .into_response()),
        constants::CSUBMISSION_TABLE_NAME => {
            let f = match &mut body.filter {
                None => return Err(ErrorResponse::bad_request_with_str("参数错误")),
                Some(x) => x,
            };
            let cid = match f.remove("cid") {
                None => return Err(ErrorResponse::bad_request_with_str("参数错误")),
                Some(v) => v.as_i64().unwrap(),
            };
            //判断操作合法性
            Ok(Response::from(
                csd::find(cid, body.filter, body.desc, body.limit, body.offset).await?,
            )
                .into_response())
        }
        constants::TSUBMISSION_TABLE_NAME => Ok(Response::from(
            tsd::find(body.filter, body.desc, body.limit, body.offset).await?,
        )
            .into_response()),
        constants::POST_TABLE_NAME => {
            let f = match &mut body.filter {
                None => return Err(ErrorResponse::bad_request_with_str("参数错误")),
                Some(x) => x,
            };
            let cid = f.remove("cid").unwrap_or(json!(0)).as_i64().unwrap_or(0);
            let pid = match f.remove("pid") {
                None => return Err(ErrorResponse::bad_request_with_str("参数错误")),
                Some(v) => {
                    match v.as_i64() {
                        Some(x) => x,
                        _ => return Err(ErrorResponse::bad_request_with_str("参数错误")),
                    }
                }
            };

            let uid = match token_data {
                None => 0,
                Some(x) => x.id
            };
            if cid != 0 && uid != 1 && cd::allowed_enter_of_user(cid, uid).await.is_err() {
                return Err(ErrorResponse::bad_request_with_str("无权获取"))
            }
            Ok(Response::from(post_dao::find(cid, pid, body.desc, body.limit, body.offset).await?).into_response())
        }
        constants::COMMENT_TABLE_NAME => {
            let f = match &mut body.filter {
                None => return Err(ErrorResponse::bad_request_with_str("参数错误")),
                Some(x) => x,
            };
            let post_id = match f.remove("post_id") {
                None => return Err(ErrorResponse::bad_request_with_str("参数错误")),
                Some(v) => {
                    match v.as_i64() {
                        Some(x) => x,
                        _ => return Err(ErrorResponse::bad_request_with_str("参数错误")),
                    }
                }
            };
            let cid = crud::get_one_value::<Post, i64>(post_id, "cid").await?;
            let uid = match token_data {
                None => 0,
                Some(x) => x.id
            };
            if cid != 0 && uid != 1 && cd::allowed_enter_of_user(cid, uid).await.is_err() {
                return Err(ErrorResponse::bad_request_with_str("无权获取"))
            }
            Ok(Response::from(post_dao::find_comments(post_id, body.limit, body.offset).await?).into_response())
        }
        _ => Err(ErrorResponse::not_found_with_str("not found")),
    }
}

pub async fn count(
    Path(table_name): Path<String>,
    Query(mut query): Query<JsonMap>,
    token_data: Option<Extension<jwt::UserToken>>,
) -> ResponseResult {
    let mut filter = match query.remove("filter") {
        Some(v) => serde_json::from_str(v.as_str().unwrap()).unwrap_or(JsonMap::new()),
        None => JsonMap::new(),
    };
    let res = match table_name.as_str() {
        constants::USER_TABLE_NAME => ud::count(Some(filter)).await?,
        constants::PROBLEM_TABLE_NAME => {
            let is_open = match filter.remove("is_open") {
                None => false,
                Some(v) => v.as_bool().unwrap_or(false),
            };
            if !is_open && (token_data.is_none() || !token_data.unwrap().is_admin()) {
                return Err(ErrorResponse::unauthorized_default());
            }
            pd::count(
                is_open,
                match filter.is_empty() {
                    true => None,
                    false => Some(filter),
                },
            )
                .await?
        }
        constants::SUBMISSION_TABLE_NAME => sd::count(Some(filter)).await?,
        constants::CONTEST_TABLE_NAME => cd::count(Some(filter)).await?,
        constants::CSUBMISSION_TABLE_NAME => {
            let cid = match filter.remove("cid") {
                None => 0,
                Some(v) => v.as_i64().unwrap_or(0),
            };
            //判断操作合法性
            csd::count(cid, Some(filter)).await?
        }
        constants::TSUBMISSION_TABLE_NAME => tsd::count(Some(filter)).await?,
        _ => 0,
    };
    Ok(Response::from(res).into_response())
}

//需要super_admin
pub async fn update(Path((table_name, id)): Path<(String, i64)>, AxumJson(mp): AxumJson<JsonMap>) -> ResponseResult {
    let res = match table_name.as_str() {
        constants::PROBLEM_TABLE_NAME => pd::update(id, mp).await?,
        constants::CONTEST_TABLE_NAME => cd::update(id, mp).await?,
        _ => (),
    };
    Ok(Response::from(res).into_response())
}

#[derive(Deserialize)]
pub struct GetBody {
    wants: Vec<String>,
    id: i64,
}

pub async fn get(Path(table_name): Path<String>, AxumJson(body): AxumJson<GetBody>, token_data: Option<Extension<jwt::UserToken>>,) -> ResponseResult {
    if body.wants.is_empty() {
        return Ok(Response::from(JsonMap::new()).into_response());
    }
    let wants: Vec<_> = body.wants.iter().map(|s| s.as_str()).collect();

    let res = match table_name.as_str() {
        constants::USER_TABLE_NAME => ud::get_fields(body.id, wants).await?,
        constants::PROBLEM_TABLE_NAME => {
            let is_open = crud::get_one_value::<Problem, bool>(body.id, "is_open").await?;
            if !is_open && (token_data.is_none() || !token_data.unwrap().is_super_admin()) {
                return Err(ErrorResponse::bad_request_with_str("没有权限!!!"))
            }
            pd::get_fields(body.id, wants).await?
        },
        constants::SUBMISSION_TABLE_NAME => {
            let values = crud::get_values::<Submission>(body.id, &["uid", "is_open"]).await?;
            let uid = values[0].as_i64().unwrap_or(0);
            let is_open = values[1].as_bool().unwrap_or(false);
            if !is_open {
                if token_data.is_none() {
                    return Err(ErrorResponse::bad_request_with_str("没有权限!!!"))
                }
                let tk = token_data.unwrap();
                if !tk.is_super_admin() && tk.id != uid {
                    return Err(ErrorResponse::bad_request_with_str("没有权限!!!"))
                }
            }
            sd::get_fields(body.id, wants).await?
        },
        constants::CONTEST_TABLE_NAME => cd::get_fields(body.id, wants).await?,
        _ => JsonMap::new(),
    };
    Ok(Response::from(res).into_response())
}