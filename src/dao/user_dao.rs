use super::{crud, redis_db, postgres};
use crate::dao::crud::base_count;
use crate::model::problem::Problem;
use crate::model::{
    submission::Submission,
    traits::*,
    user::User,
};
use crate::types::links::JsonMap;
use crate::types::status as Status;
use anyhow::Result;
use serde_json::json;

pub fn submission_list_key(uid: i64, pid: i64) -> String {
    format!("submissions_uid:{}_pid:{}", uid, pid)
}

pub async fn get_submissions(uid: i64, pid: i64) -> Result<Vec<JsonMap>> {
    let key = submission_list_key(uid, pid);
    if redis_db::exists(key.as_str()).await? {
        let val = redis_db::get(key).await?;
        return serde_json::from_value(val).map_err(|e| anyhow::Error::msg(format!("{}", e)));
    }

    let username = crud::get_one_value::<User, String>(uid, "username").await?;
    let tmp = crud::get_values::<Problem>(pid, &["title", "index"]).await?;
    let title = tmp[0].as_str().unwrap().to_string();
    let index = tmp[1].as_str().unwrap().to_string();

    let conn = postgres::get_pg_connect().await?;
    let fields: &[&str] = Submission::zset_fields();
    let sql = format!(
        "SELECT {} FROM {} WHERE pid = {} AND uid = {} ORDER BY id DESC",
        fields.join(","),
        Submission::table_name(),
        pid,
        uid,
    );

    let res: Vec<JsonMap> = conn.query(sql.as_str(), &[]).await.and_then(|rows| {
        Ok(rows.into_iter().map(|row| {
            let mut mp = crud::pg_row_to_json_map(row);
            mp.insert("username".into(), json!(username));
            mp.insert("title".into(), json!(title));
            mp.insert("index".into(), json!(index));
            mp
        }).collect())
    })?;
    redis_db::set(key, &json!(res), 3600 * 8).await?;
    Ok(res)
}

pub async fn deal_submission_result(uid: i64, pid: i64, status: i32) -> Result<()> {
    redis_db::del(submission_list_key(uid, pid))
        .await
        .unwrap_or(());

    let v = crud::get_values::<User>(
        uid,
        &[
            "solved",
            "all_count",
            "accepted_count",
            "solved_problems",
            "failed_problems",
            "id",
        ],
    )
    .await?;

    let solved = v[0].as_i64().unwrap();
    let all_count = v[1].as_i64().unwrap();
    let accepted_count = v[2].as_i64().unwrap();
    let id = v[5].as_i64().unwrap();
    let org_score = User::redis_score(id, solved, accepted_count, all_count);
    let zkey = User::zset_key();
    redis_db::zrembyscore(zkey, org_score, org_score).await?;

    let mut mp = crate::json_map!("all_count" => all_count + 1);
    let mut solve_problems: Vec<i64> = serde_json::from_value(v[3].clone()).unwrap_or(vec![]);
    if status == Status::AC {
        mp.insert("accepted_count".into(), json!(accepted_count + 1));
        if !solve_problems.contains(&pid) {
            solve_problems.push(pid);
            mp.insert("solved_problems".into(), json!(solve_problems));
            mp.insert("solved".into(), json!(solve_problems.len()));
            let mut failed_problems: Vec<i64> =
                serde_json::from_value(v[4].clone()).unwrap_or(vec![]);
            let mut existed = false;
            for i in 0..failed_problems.len() {
                if failed_problems[i] == pid {
                    existed = true;
                    failed_problems.remove(i);
                    break;
                }
            }
            if existed {
                mp.insert("failed_problems".into(), json!(failed_problems));
            }
        }
    } else {
        if !solve_problems.contains(&pid) {
            let mut failed_problems: Vec<i64> =
                serde_json::from_value(v[4].clone()).unwrap_or(vec![]);
            if !failed_problems.contains(&pid) {
                failed_problems.push(pid);
                mp.insert("failed_problems".into(), json!(failed_problems));
            }
        }
    }
    crud::update_by_map_without_dev::<User>(uid, &mp).await?;

    Ok(())
}

pub async fn find(
    filter: Option<JsonMap>,
    desc: Option<bool>,
    limit: Option<i32>,
    offset: Option<i32>,
) -> Result<Vec<JsonMap>> {
    crud::base_find::<User, i64>(filter, desc, limit, offset, None).await
}

pub async fn count(filter: Option<JsonMap>) -> Result<i64> {
    base_count::<User, i64>(filter, None).await
}

pub async fn get_fields(id: i64, wants: Vec<&str>) -> Result<JsonMap> {
    crud::get_columns::<User>(id, wants.as_slice()).await
}
