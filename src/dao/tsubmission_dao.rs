use super::{
    crud, redis_db, postgres,
    submission_dao::{make_judge_config, make_update_map, TMP_STATUS_EXPIRE},
};
use crate::constants;
use crate::model::tsubmission::TSubmission;
use crate::types::links::JsonMap;
use crate::types::status as Status;
use crate::utils::judger::{acquire_judge_chance, judge, release_judge_chance};
use anyhow::Result;
use serde_json::json;
use std::collections::HashMap;

pub fn tmp_tsm_status_key(sid: i64) -> String {
    format!("status_of_tsm:{}", sid)
}

pub async fn handle_tsubmission(uid: i64, pid: i64, lang: String, code: String) -> Result<i64> {
    let mut s = TSubmission::from(uid, pid, lang, code);
    s.id = crud::insert::<TSubmission, i64>(&s, Some(pid)).await?;
    let sid = s.id;
    tokio::spawn(async move {
        let sm_status_key = tmp_tsm_status_key(s.id);
        redis_db::set(
            sm_status_key.as_str(),
            &json!(Status::QUEUEING),
            TMP_STATUS_EXPIRE,
        )
        .await
        .unwrap();
        acquire_judge_chance().await;
        redis_db::set(
            sm_status_key.as_str(),
            &json!(Status::RUNNING),
            TMP_STATUS_EXPIRE,
        )
        .await
        .unwrap();

        let jc = make_judge_config(s.pid, s.lang.clone(), s.code, true)
            .await
            .unwrap();
        //代码运行
        let res = judge(jc).await.unwrap();
        release_judge_chance().await;
        let update_map = make_update_map(&res);

        let sid = s.id;
        let pid = s.pid;
        let status = res.status;

        //更新提交
        redis_db::set(sm_status_key.as_str(), &json!(status), TMP_STATUS_EXPIRE)
            .await
            .unwrap_or(());
        crud::update_by_map::<TSubmission, i64>(
            sid,
            &crate::json_map!("result" => json!(update_map).to_string()),
            Some(pid),
        )
        .await
        .unwrap();
    });

    Ok(sid)
}

pub async fn find(
    filter: Option<JsonMap>,
    desc: Option<bool>,
    limit: Option<i32>,
    offset: Option<i32>,
) -> Result<Vec<JsonMap>> {
    let filter = filter.unwrap_or(JsonMap::new());
    let problem_exist = filter.contains_key("index");
    let ts = match (problem_exist, filter.len()) {
        (true, 1) => {
            let index = filter.get("index").unwrap().as_str().unwrap();
            let pid = super::problem_dao::get_problem_id(index).await?;
            crud::zfind::<TSubmission, i64>(desc, limit, offset, Some(pid)).await?
        }
        _ => crud::find_base_columns::<TSubmission>(Some(filter), desc, limit, offset).await?,
    };
    if ts.is_empty() {
        return Ok(vec![]);
    }
    let pids: Vec<_> = ts.iter().map(|x| x.get("pid").unwrap().to_string()).collect();
    let psql = format!(
        "SELECT id,title,index FROM \"{}\" WHERE id in ({})",
        constants::PROBLEM_TABLE_NAME,
        pids.join(",")
    );

    let conn = postgres::get_pg_connect().await.unwrap();
    let rows = conn.query(psql.as_str(), &[]).await.unwrap();
    let mut title_mp: HashMap<i64, String> = HashMap::new();
    let mut index_mp: HashMap<i64, String> = HashMap::new();
    for row in rows {
        let id: i64 = row.get("id");
        title_mp.insert(id, row.get("title"));
        index_mp.insert(id, row.get("index"));
    }

    let ret: Vec<_> = ts
        .into_iter()
        .map(|x| {
            let pid = x.get("pid").unwrap().as_i64().unwrap();
            match title_mp.get(&pid) {
                Some(t) => {
                    let mut mp: JsonMap = x.into();
                    mp.insert("title".into(), json!(t));
                    mp.insert("index".into(), json!(index_mp.get(&pid).unwrap()));
                    mp
                },
                None => JsonMap::new()
            }
        })
        .filter(|mp| !mp.is_empty())
        .collect();
    Ok(ret)
}

pub async fn count(filter: Option<JsonMap>) -> Result<i64> {
    let filter = filter.unwrap_or(JsonMap::new());
    match filter.contains_key("index") && filter.len() == 1 {
        true => {
            let index = filter.get("index").unwrap().as_str().unwrap();
            let pid = super::problem_dao::get_problem_id(index).await?;
            crud::zcard::<TSubmission, i64>(Some(pid)).await
        }
        false => crud::count::<TSubmission>(filter).await,
    }
}

pub async fn get_status(sid: i64) -> Result<i32> {
    let key = tmp_tsm_status_key(sid);
    let res = redis_db::get(key).await?;
    Ok(res.as_i64().unwrap_or(0) as i32)
}
