use super::{crud, redis_db, postgres, problem_dao as pd, user_dao as ud};
use crate::constants;
use crate::json_map;
use crate::model::problem::Problem;
use crate::model::submission::Submission;
use crate::types::links::JsonMap;
use crate::types::status as Status;
use crate::utils::judger::{
    acquire_judge_chance, judge, release_judge_chance, JudgeConfig, JudgeResult,
};
use anyhow::Result;
use serde_json::json;
use std::collections::HashMap;

pub const TMP_STATUS_EXPIRE: usize = 120;

pub fn tmp_sm_status_key(sid: i64) -> String {
    format!("status_of_sm:{}", sid)
}
pub async fn make_judge_config(pid: i64, lang: String, code: String, test_all: bool) -> Result<JudgeConfig> {
    let p = crud::get_values::<Problem>(
        pid,
        &[
            "index",
            "time_limit",
            "memory_limit",
            "test_cases",
            "checker",
            "spj_config",
        ],
    )
    .await?;
    Ok(JudgeConfig {
        lang,
        src: code,
        io_dir: format!("{}", p[0].as_str().unwrap()),
        max_cpu_time: p[1].as_i64().unwrap() as i32,
        max_memory: p[2].as_i64().unwrap() as i32 * 1024 * 1024, //MB -> byte
        test_cases: serde_json::from_value(p[3].clone()).unwrap(),
        checker: serde_json::from_value(p[4].clone()).unwrap(),
        spj_config: serde_json::from_value(p[5].clone()).unwrap(),
        seccomp_rule: None,
        resource_rule: None,
        test_all: Some(test_all)
    })
}

pub fn make_update_map(res: &JudgeResult) -> JsonMap {
    json_map!(
        "compile_info" => res.compile_info.replace("'", "''"),
        "case_count" => res.case_count,
        "pass_count" => res.pass_count,
        "time" => res.time,
        "memory" => res.memory,
        "total_time" => res.total_time,
        "status" => res.status,
        "error" => res.error,
        "details" => res.results
    )
}
pub async fn handle_submission(uid: i64, pid: i64, lang: String, code: String) -> Result<i64> {
    // ;
    let mut s = Submission::from(uid, pid, lang, code);
    s.id = crud::insert::<Submission, i64>(&s, Some(pid)).await?;
    let sid = s.id;

    tokio::spawn(async move {
        let sm_status_key = tmp_sm_status_key(s.id);
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

        let _sid = s.id;
        let _pid = s.pid;
        let jc = make_judge_config(s.pid, s.lang.clone(), s.code, false)
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
        tokio::spawn(async move {
            redis_db::set(sm_status_key.as_str(), &json!(status), TMP_STATUS_EXPIRE)
                .await
                .unwrap_or(());
            crud::update_by_map::<Submission, i64>(sid, &update_map, Some(pid))
                .await
                .unwrap();
        });

        let pid = s.pid;
        let status = res.status;
        //更新题目
        tokio::spawn(async move {
            let is_open = crud::get_one_value::<Problem, bool>(pid, "is_open")
                .await
                .unwrap();
            if status == Status::AC {
                crud::inc::<Problem, bool>(pid, &["accepted_count", "all_count"], Some(is_open))
                    .await
                    .unwrap();
            } else {
                crud::inc::<Problem, bool>(pid, &["all_count"], Some(is_open))
                    .await
                    .unwrap();
            }
        });

        let uid = s.uid;
        let pid = s.pid;
        let status = res.status;
        //更新用户
        tokio::spawn(async move {
            ud::deal_submission_result(uid, pid, status)
                .await
                .unwrap_or(());
        });
    });
    Ok(sid)
}

pub async fn get_submission_status(sid: i64) -> Result<i32> {
    let sm_status_key = tmp_sm_status_key(sid);
    let res = redis_db::get(sm_status_key).await?;
    if res.is_null() {
        crud::get_one_value::<Submission, i32>(sid, "status").await
    } else {
        Ok(res.as_i64().unwrap() as i32)
    }
}

pub async fn get_last_submit(uid: i64, pid: i64) -> Result<JsonMap> {
    let conn = postgres::get_pg_connect().await?;
    let sql = format!(
        "SELECT id,code,lang from {} WHERE pid='{}' AND uid='{}' ORDER BY ID DESC LIMIT 1",
        constants::SUBMISSION_TABLE_NAME,
        pid,
        uid
    );

    let q = conn.query_one(sql.as_str(), &[]).await;
    Ok(match q {
        Ok(row) => json_map!(
            "id" => row.get::<&str, i64>("id"),
            "code" => row.get::<&str, String>("code"),
            "lang" => row.get::<&str, String>("lang")
        ),
        Err(_) => JsonMap::new(),
    })
}

pub async fn get_fields(id: i64, wants: Vec<&str>) -> Result<JsonMap> {
    crud::get_columns::<Submission>(id, wants.as_slice()).await
}

pub async fn make_filter(filter: Option<JsonMap>) -> Result<(bool, JsonMap)> {
    let mut filter = filter.unwrap_or(JsonMap::new());
    if let Some(v) = filter.remove("username") {
        let conn = postgres::get_pg_connect().await?;
        let sql = format!(
            "SELECT id from \"{}\" WHERE username = '{}'",
            constants::USER_TABLE_NAME,
            v.as_str().unwrap_or("")
        );
        let v = conn.query(sql.as_str(), &[]).await?;
        if v.is_empty() {
            return Ok((false, filter));
        } else {
            let id: i64 = v[0].get("id");
            filter.insert("uid".to_string(), json!(id));
        }
    }
    if let Some(v) = filter.remove("index") {
        let id = pd::get_problem_id(v.as_str().unwrap_or("")).await;
        if id.is_err() {
            return Ok((false, filter));
        }
        filter.insert("pid".to_string(), json!(id.unwrap()));
    }
    Ok((true, filter))
}

pub async fn find(
    filter: Option<JsonMap>,
    desc: Option<bool>,
    limit: Option<i32>,
    offset: Option<i32>,
) -> Result<Vec<JsonMap>> {
    let (ok, filter) = make_filter(filter).await?;
    if !ok {
        return Ok(vec![]);
    }
    let pid_exist = filter.contains_key("pid");
    let uid_exist = filter.contains_key("uid");

    let ss = match (pid_exist, uid_exist, filter.len()) {
        (true, false, 1) => {
            let pid = filter.get("pid").unwrap().as_i64().unwrap();
            crud::zfind::<Submission, i64>(desc, limit, offset, Some(pid)).await
        }
        (true, true, 2) => {
            return ud::get_submissions(
                filter.get("uid").unwrap().as_i64().unwrap(),
                filter.get("pid").unwrap().as_i64().unwrap(),
            )
            .await
        }
        _ => crud::find_base_columns::<Submission>(Some(filter), desc, limit, offset).await,
    }?;
    if ss.is_empty() {
        return Ok(vec![]);
    }
    let uids: Vec<_> = ss.iter().map(|x| x.get("uid").unwrap().to_string()).collect();
    let pids: Vec<_> = ss.iter().map(|x| x.get("pid").unwrap().to_string()).collect();
    let usql = format!(
        "SELECT id,username FROM \"{}\" WHERE id in ({})",
        constants::USER_TABLE_NAME,
        uids.join(",")
    );
    let psql = format!(
        "SELECT id,title,index FROM \"{}\" WHERE id in ({})",
        constants::PROBLEM_TABLE_NAME,
        pids.join(",")
    );

    let u_task = tokio::spawn(async move {
        let conn = postgres::get_pg_connect().await.unwrap();

        let rows = conn.query(usql.as_str(), &[]).await.unwrap();
        let mut ump: HashMap<i64, String> = HashMap::new();
        for row in rows {
            ump.insert(row.get("id"), row.get("username"));
        }
        ump
    });

    let p_task = tokio::spawn(async move {
        let conn = postgres::get_pg_connect().await.unwrap();
        let rows = conn.query(psql.as_str(), &[]).await.unwrap();
        let mut title_mp: HashMap<i64, String> = HashMap::new();
        let mut index_mp: HashMap<i64, String> = HashMap::new();
        for row in rows {
            let id: i64 = row.get("id");
            title_mp.insert(id, row.get("title"));
            index_mp.insert(id, row.get("index"));
        }
        (title_mp, index_mp)
    });

    let res = tokio::join!(u_task, p_task);

    let ump = res.0?;
    let (title_mp, index_mp) = res.1?;
    let ret: Vec<_> = ss
        .into_iter()
        .map(|x| {
            let pid = x.get("pid").unwrap().as_i64().unwrap();
            let uid = x.get("uid").unwrap().as_i64().unwrap();
            match title_mp.get(&pid) {
                Some(title) => {
                    match ump.get(&uid) {
                        Some(username) => {
                            let mut mp: JsonMap = x.into();
                            mp.insert("username".into(), json!(username));
                            mp.insert("title".into(), json!(title));
                            mp.insert("index".into(), json!(index_mp.get(&pid).unwrap()));
                            mp
                        }
                        None => JsonMap::new()
                    }
                }
                None => JsonMap::new()
            }
        })
        .filter(|mp| !mp.is_empty())
        .collect();
    Ok(ret)
}

pub async fn count(filter: Option<JsonMap>) -> Result<i64> {
    let (ok, filter) = make_filter(filter).await?;
    if !ok {
        return Ok(0);
    }
    match filter.contains_key("pid") && filter.len() == 1 {
        true => {
            let pid = filter.get("pid").unwrap().as_i64().unwrap();
            crud::zcard::<Submission, i64>(Some(pid)).await
        }
        false => crud::count::<Submission>(filter).await,
    }
}
