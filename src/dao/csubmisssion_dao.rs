use crate::dao::crud;
use crate::model::csubmission::{CSubmission};
use crate::types::links::JsonMap;
use anyhow::Result;
use serde_json::json;

pub async fn find(
    cid: i64,
    filter: Option<JsonMap>,
    desc: Option<bool>,
    limit: Option<i32>,
    offset: Option<i32>,
) -> Result<Vec<JsonMap>> {
    let mut filter = filter.unwrap_or(JsonMap::new());
    match filter.is_empty() {
        true => crud::zfind::<CSubmission, i64>(desc, limit, offset, Some(cid)).await,
        false => {
            filter.insert("cid".into(), json!(cid));
            crud::find_base_columns::<CSubmission>(Some(filter), desc, limit, offset).await
        }
    }
}

pub async fn count(cid: i64, filter: Option<JsonMap>) -> Result<i64> {
    let mut filter = filter.unwrap_or(JsonMap::new());
    match filter.is_empty() {
        true => crud::zcard::<CSubmission, i64>(Some(cid)).await,
        false => {
            filter.insert("cid".into(), json!(cid));
            crud::count::<CSubmission>(filter).await
        }
    }
}
