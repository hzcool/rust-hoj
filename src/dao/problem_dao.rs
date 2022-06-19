use super::crud;
use crate::constants;
use crate::model::problem::{Problem, TestCase};
use crate::model::traits::*;
use crate::types::links::JsonMap;
use anyhow::Result;
use serde_json::{json, Value as Json};

use futures::StreamExt;
use std::ffi::OsStr;
use std::path::Path;
use crate::constants::ZIPPER_PATH;

pub fn score_of_index(index: &str) -> i64 {
    match index[1..].parse::<i64>() {
        Err(_) => -1,
        Ok(x) => x,
    }
}
pub fn is_valid_index(index: &str) -> bool {
    score_of_index(index) != -1
}
fn is_open(index: &str) -> bool {
    match &index[0..1] {
        "P" => true,
        _ => false,
    }
}

pub fn get_last_index_mapper_key(is_open: bool) -> &'static str {
    match is_open {
        true => "pub_last_p_idx",
        false => "pri_last_p_idx",
    }
}

pub async fn last_index(is_open: bool) -> Result<String> {
    let mapper_key = get_last_index_mapper_key(is_open);
    let idx_json = crud::get_mapper(mapper_key).await?;
    Ok(match serde_json::from_value(idx_json) {
        Ok(idx) => idx,
        Err(_) => {
            let mut score: i64 = 999;
            let res = crud::zrange::<Problem, bool>(-1, -1, Some(is_open)).await?;
            if !res.is_empty() {
                score = score_of_index(res[0].get("index").unwrap().as_str().unwrap())
            }
            let idx = match is_open {
                true => format!("P{}", score),
                false => format!("U{}", score),
            };
            crud::add_mapper(mapper_key, json!(idx)).await.unwrap_or(());
            idx
        }
    })
}

pub async fn get_problem_id(index: &str) -> Result<i64> {
    let id = crud::get_mapper(index).await?;
    Ok(match serde_json::from_value(id) {
        Ok(x) => x,
        Err(_) => {
            let score = score_of_index(index);
            let res =
                crud::zrangebyscore::<Problem, bool>(score, score, Some(is_open(index))).await?;
            if res.is_empty() {
                Err(anyhow::Error::msg("not found"))?
            }
            let id = res[0].get("id").unwrap();
            crud::add_mapper(index, id.clone())
                .await
                .unwrap_or(());
            id.as_i64().unwrap()
        }
    })
}

pub async fn get_many_indexes(ids: &Vec<i64>) -> Result<Vec<String>> {
    if ids.is_empty() {
        return Ok(vec![]);
    }
    let sql = format!(
        "SELECT index FROM \"{}\" WHERE id in ({}) ORDER BY id ASC",
        constants::PROBLEM_TABLE_NAME,
        ids.iter()
            .map(|id| format!("{}", id))
            .collect::<Vec<_>>()
            .join(","),
    );
    let conn = super::postgres::get_pg_connect().await?;
    Ok(conn
        .query(sql.as_str(), &[])
        .await?
        .into_iter()
        .map(|r| r.get("index"))
        .collect())
}

pub async fn find(
    is_open: bool,
    filter: Option<JsonMap>,
    desc: Option<bool>,
    limit: Option<i32>,
    offset: Option<i32>,
) -> Result<Vec<JsonMap>> {
    let mut filter = filter.unwrap_or(JsonMap::new());
    if !filter.is_empty() {
        filter.insert("is_open".into(), Json::Bool(is_open));
    }
    crud::base_find::<Problem, bool>(Some(filter), desc, limit, offset, Some(is_open)).await
}

pub async fn count(is_open: bool, filter: Option<JsonMap>) -> Result<i64> {
    let mut filter = filter.unwrap_or(JsonMap::new());
    if !filter.is_empty() {
        filter.insert("is_open".into(), Json::Bool(is_open));
    }
    crud::base_count::<Problem, bool>(Some(filter), Some(is_open)).await
}

pub async fn update(id: i64, mp: JsonMap) -> Result<()> {
    if mp.contains_key("index") || mp.contains_key("id") {
        return Err(anyhow::Error::msg("题号不能更改"));
    }
    if mp.contains_key("is_open") {
        return Err(anyhow::Error::msg("题目公开性不能修改， 但是可以拷贝"));
    }

    let is_open = crud::get_one_value::<Problem, bool>(id, "is_open").await?;
    crud::update_by_map::<Problem, bool>(id, &mp, Some(is_open)).await
}

pub async fn get_fields(id: i64, wants: Vec<&str>) -> Result<JsonMap> {
    if wants[0] == "*" {
        return crud::get_columns::<Problem>(id, Problem::field_names()).await;
    }
    crud::get_columns::<Problem>(id, wants.as_slice()).await
}

// pub async fn exist_problem(index : &str) -> Result<bool> {
//     let x = get_problem_id(index).await.unwrap_or(0);
//     Ok(x > 0)
// }

pub async fn insert(p: Problem) -> Result<i64> {
    if !is_valid_index(p.index.as_str()) {
        return Err(anyhow::Error::msg("题号设置错误"));
    }
    if p.is_open != is_open(p.index.as_str()) {
        return Err(anyhow::Error::msg("题号不符合题目公开性要求"));
    }
    if get_problem_id(p.index.as_str()).await.is_ok() {
        return Err(anyhow::Error::msg("题号已存在"));
    }
    crud::del_mapper(get_last_index_mapper_key(p.is_open)).await?;
    Ok(crud::insert::<Problem, bool>(&p, Some(p.is_open)).await?)
}

pub async fn delete(pid: i64) -> Result<()> {
    let v = crud::get_values::<Problem>(pid, &["index", "is_open"]).await?;
    let is_open = v[1].as_bool().unwrap();
    crud::delete::<Problem, bool>(pid, Some(is_open)).await?;
    crud::del_mapper(get_last_index_mapper_key(is_open)).await?;
    crud::del_mapper(v[0].as_str().unwrap()).await
}

pub async fn handle_zip_data(index: &str, zip_path: &Path) -> Result<Vec<TestCase>> {
    let target_dir = Path::new(constants::TEST_CASE_DIR.as_str()).join(index);
    async_process::Command::new(ZIPPER_PATH.as_os_str())
        .arg("unzip")
        .arg(zip_path.as_os_str())
        .arg(target_dir.as_os_str())
        .output()
        .await?;
    make_test_case_info(index, target_dir.as_path()).await
}

pub async fn make_test_case_info(index: &str, dir: &Path) -> Result<Vec<TestCase>> {
    let _dir = Path::new(dir);
    let (input, mut output) = futures::executor::block_on(async move {
        let mut input = std::collections::BTreeMap::new();
        let mut output = std::collections::BTreeMap::new();
        let mut entries = async_walkdir::WalkDir::new(_dir);
        loop {
            match entries.next().await {
                Some(Ok(entry)) => {
                    let meta_data = entry.metadata().await.unwrap();
                    if meta_data.is_dir() {
                        continue;
                    }
                    let name = format!("{}", entry.file_name().to_str().unwrap());
                    let e = name.rfind(".").unwrap_or(0);
                    let s = match (&name[0..e]).rfind(|c: char| !c.is_numeric()) {
                        None => 0,
                        Some(x) => x + 1,
                    };
                    let id = name[s..e].parse::<i32>().unwrap_or(0);
                    if entry.path().extension().unwrap_or(&OsStr::new("")) == "in" {
                        input.insert(id, (name, meta_data.len() as i32));
                    } else {
                        output.insert(id, (name, meta_data.len() as i32));
                    }
                }
                _ => break,
            }
        }
        (input, output)
    });
    let mut test_cases = Vec::new();
    let mut cnt: i32 = 0;
    for (id, (input_name, input_size)) in input {
        if let Some((output_name, output_size)) = output.remove(&id) {
            cnt += 1;
            test_cases.push(TestCase {
                id: cnt,
                input_name,
                input_size,
                output_name,
                output_size,
                max_output_size: None,
            })
        }
    }
    let mut mp = JsonMap::new();
    mp.insert("test_cases".to_string(), json!(test_cases));
    update(get_problem_id(index).await?, mp).await?;
    Ok(test_cases)
}
