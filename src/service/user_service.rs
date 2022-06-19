use crate::{constants, json_map};
use crate::dao::{
    contest_dao as cd, crud, csubmisssion_dao as csd, postgres, problem_dao as pd,
    submission_dao as sd, post_dao
};
use crate::model::{
    contest::{
        self,
        Contest,
        ContestProblem
    },
    csubmission::CSubmission,
    team::Team,
    user::User,
};
use crate::types::{
    links::{JsonMap, ResponseResult},
    error_response::ErrorResponse,
    response::Response,
};
use crate::utils::jwt::UserToken;
use serde::Deserialize;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use axum::{
    extract::{Path, Extension, Query, Json as AxumJson},
    response::IntoResponse
};
use crate::dao::contest_dao::allowed_enter_of_user;
use crate::model::comment::Comment;
use crate::model::post::Post;
use crate::model::problem::Problem;

pub async fn get_user_info(Path(uid): Path<i64>) -> ResponseResult {
    let u = crud::get_object::<User>(uid).await?;
    let (ac, fail) = tokio::join!(
        pd::get_many_indexes(&u.solved_problems),
        pd::get_many_indexes(&u.failed_problems)
    );
    let mut mp: JsonMap = u.into();
    mp.remove("password");
    mp.insert("solved_problems".into(), json!(ac?));
    mp.insert("failed_problems".into(), json!(fail?));
    Ok(Response::from(mp).into_response())
}

pub async fn get_practice_record(Path(uid): Path<i64>) -> ResponseResult {
    let ve = crud::get_values::<User>(uid, &["solved_problems", "failed_problems"]).await?;
    let solved: Vec<_> = ve[0]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .map(|x| x.as_i64().unwrap().clone())
        .collect();
    let failed: Vec<_> = ve[1]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .map(|x| x.as_i64().unwrap().clone())
        .collect();
    let (x, y) = tokio::join!(pd::get_many_indexes(&solved), pd::get_many_indexes(&failed));
    Ok(Response::from(crate::json_map!("solved_problems"=>x?, "failed_problems"=>y?)).into_response())
}

pub async fn get_user_all_submissions_status_info(Path(uid): Path<i64>) -> ResponseResult {
    let sql = format!(
        "SELECT status FROM \"{}\" WHERE uid = {}",
        constants::SUBMISSION_TABLE_NAME,
        uid
    );
    let conn = postgres::get_pg_connect().await?;
    let mut hmp: HashMap<i32, i32> = HashMap::new();
    conn.query(sql.as_str(), &[])
        .await
        .map_err(|err| anyhow::Error::new(err))?
        .into_iter()
        .for_each(|r| {
            let status: i32 = r.get("status");
            let x = *hmp.get(&status).unwrap_or(&0) + 1;
            hmp.insert(status, x);
        });
    Ok(Response::from(hmp).into_response())
}

#[derive(Deserialize)]
pub struct SubmitCodeBody {
    pub lang: String,
    pub code: String,
    pub pid: i64,
}

pub async fn submit_code(token_data: Extension<UserToken>, AxumJson(bd): AxumJson<SubmitCodeBody>) -> ResponseResult {
    let sid = sd::handle_submission(token_data.id, bd.pid, bd.lang, bd.code).await?;
    Ok(Response::from(sid).into_response())
}

pub async fn get_last_submit(token_data: Extension<UserToken>, Path(pid): Path<i64>) -> ResponseResult {
    Ok(Response::from(sd::get_last_submit(token_data.id, pid).await?).into_response())
}

pub async fn get_status(Path(id): Path<i64>) -> ResponseResult {
    Ok(Response::from(sd::get_submission_status(id).await?).into_response())
}

pub async fn get_user_problems_status(
    token_data: Extension<UserToken>,
    Query(mut ids_mp): Query<HashMap<String, String>>,
) -> ResponseResult {
    let ids_str = ids_mp.remove("ids").unwrap();
    let ids: Vec<i64> = serde_json::from_str(ids_str.as_str()).unwrap();
    let v =
        crud::get_values::<User>(token_data.id, &["solved_problems", "failed_problems"]).await?;
    let solve_problems: Vec<i64> = serde_json::from_value(v[0].clone()).unwrap_or(vec![]);
    let failed_problems: Vec<i64> = serde_json::from_value(v[1].clone()).unwrap_or(vec![]);

    let solved_set: HashSet<_> = solve_problems.into_iter().map(|x| x).collect();
    let failed_set: HashSet<_> = failed_problems.into_iter().map(|x| x).collect();
    let res: Vec<_> = ids
        .into_iter()
        .map(|x| {
            let mut t = 0;
            if solved_set.contains(&x) {
                t = 1;
            } else if failed_set.contains(&x) {
                t = 2;
            }
            t
        })
        .collect();
    Ok(Response::from(res).into_response())
}

pub async fn register_private_contest(
    token_data: Extension<UserToken>,
    Path((cid, password)): Path<(i64, String)>,
) -> ResponseResult {
    Ok(Response::from(
        cd::register_team(
            cid,
            token_data.id,
            token_data.user.clone(),
            password.as_str(),
        )
            .await?,
    )
        .into_response())
}

pub async fn enter_contest() -> ResponseResult {
    Ok(Response::from("ok").into_response())
}

pub async fn get_contest_info(Path(id): Path<i64>) -> ResponseResult {
    let wants: Vec<_> = vec![
        "id",
        "is_open",
        "format",
        "author",
        "title",
        "description",
        "problems",
        "status",
        "begin",
        "length",
        "team_count",
        "clarifications",
    ];
    Ok(Response::from(cd::get_fields(id, wants).await?).into_response())
}

async fn __get_cproblems_with_check(
    token_data: &Extension<UserToken>,
    cid: i64,
) -> Result<Vec<ContestProblem>, ErrorResponse> {
    let mut res = cd::get_fields(cid, vec!["problems", "status"]).await?;
    if !token_data.is_super_admin() {
        if let Some(status) = res.remove("status") {
            if status.as_i64().unwrap() == 0 {
                return Err(ErrorResponse::forbidden_with_str("比赛还未开始"));
            }
        }
    }
    Ok(serde_json::from_value(
        res.remove("problems")
            .unwrap_or(serde_json::Value::Array(vec![])),
    )
        .unwrap())
}

async fn __cproblem(
    token_data: &Extension<UserToken>,
    cid: i64,
    label: String,
) -> Result<ContestProblem, ErrorResponse> {
    let res = __get_cproblems_with_check(token_data, cid).await?;
    for item in res.into_iter() {
        if item.label == label {
            return Ok(item);
        }
    }
    Err(ErrorResponse::not_found_with_str("Not Found"))
}

pub async fn get_contest_problems(token_data: Extension<UserToken>, Path(cid): Path<i64>) -> ResponseResult {
    let mut problems = __get_cproblems_with_check(&token_data, cid).await?;
    problems.iter_mut().for_each(|item| {
        if item.all_count.is_none() {
            item.all_count = Some(0);
        }
        if item.accepted_count.is_none() {
            item.accepted_count = Some(0);
        }
    });
    Ok(Response::from(problems).into_response())
}

pub async fn get_cproblem(token_data: Extension<UserToken>, Path((cid, label)): Path<(i64, String)>) -> ResponseResult {
    let cp = __cproblem(&token_data, cid, label).await?;
    let wants = vec![
        "id",
        "title",
        "background",
        "statement",
        "input",
        "output",
        "hint",
        "examples_in",
        "examples_out",
        "time_limit",
        "memory_limit",
    ];
    let res = pd::get_fields(cp.pid, wants).await?;
    Ok(Response::from(res).into_response())
}

#[derive(Deserialize)]
pub struct SubmitContestCodeBody {
    pub lang: String,
    pub code: String,
    pub label: String,
}

pub async fn submit_contest_code(
    token_data: Extension<UserToken>,
    Path(cid): Path<i64>,
    AxumJson(body): AxumJson<SubmitContestCodeBody>,
) -> ResponseResult {
    let values = crud::get_values::<Contest>(cid, &["status", "format"]).await?;
    let status = values[0].as_i64().unwrap() as i16;
    let format = values[1].as_str().unwrap();
    if status == contest::ENDED {
        return Err(ErrorResponse::forbidden_with_str("比赛已结束"));
    }

    let cp = __cproblem(&token_data, cid, body.label.clone()).await?;
    let tid = match cd::get_team_id(cid, token_data.id).await {
        Ok(t) => t,
        Err(_) => {
            cd::register_team(cid, token_data.id, token_data.user.clone(), "")
                .await?
                .id
        }
    };
    let sid = cd::handle_contest_submission(tid, cid, cp, body.lang, body.code, match format{
        "ACM" => false,
        _ => true
    }).await?;
    Ok(Response::from(sid).into_response())
}

pub async fn get_team(token_data: Extension<UserToken>, Path(cid): Path<i64>) -> ResponseResult {
    let tid = cd::get_team_id(cid, token_data.id).await.unwrap_or(0);
    if tid == 0 {
        return Ok(Response::from(JsonMap::new()).into_response());
    }
    Ok(Response::from(crud::get_object::<Team>(tid).await?).into_response())
}

pub async fn get_cs_status(Path((_cid, sid)): Path<(i64, i64)>) -> ResponseResult {
    Ok(Response::from(cd::get_csm_status(sid).await?).into_response())
}

pub async fn get_last_cs(token_data: Extension<UserToken>, Path((cid, label)): Path<(i64, String)>) -> ResponseResult {
    let tid = cd::get_team_id(cid, token_data.id).await.unwrap_or(0);
    if tid == 0 {
        return Ok(Response::from(JsonMap::new()).into_response());
    }

    let conn = postgres::get_pg_connect().await?;
    let sql = format!("SELECT id,code,lang from {} WHERE cid={} AND tid={} AND label = '{}' ORDER BY ID DESC LIMIT 1",
                      constants::CSUBMISSION_TABLE_NAME, cid, tid, label);
    let q = conn.query_one(sql.as_str(), &[]).await;
    let data = match q {
        Ok(row) => crate::json_map!(
            "id" => row.get::<&str, i64>("id"),
            "code" => row.get::<&str, String>("code"),
            "lang" => row.get::<&str, String>("lang")
        ),
        Err(_) => JsonMap::new(),
    };
    Ok(Response::from(data).into_response())
}

pub async fn find_team_csubmissions(Path((cid, tid, label)): Path<(i64, i64, String)>) -> ResponseResult {
    let res = crud::find_base_columns::<CSubmission>(
        Some(crate::json_map!("cid"=>cid, "tid"=>tid, "label"=>label)),
        Some(true),
        None,
        None,
    )
        .await?;
    Ok(Response::from(res).into_response())
}

pub async fn show_csubmission(token_data: Extension<UserToken>, Path((cid, sid)): Path<(i64, i64)>) -> ResponseResult {
    let wants = &[
        "id",
        "tid",
        "cid",
        "author",
        "time",
        "lang",
        "is_open",
        "memory",
        "length",
        "created_at",
        "compile_info",
        "code",
        "status",
        "case_count",
        "pass_count",
        "details",
    ];
    let mut res = crud::get_columns::<CSubmission>(sid, wants).await?;
    let tid = cd::get_team_id(cid, token_data.id).await.unwrap_or(0);

    if tid == res.get("tid").unwrap().as_i64().unwrap()
        || token_data.id == 1
        || res.get("is_open").unwrap().as_bool().unwrap()
    {
        let status = crud::get_one_value::<Contest, i16>(cid, "status").await?;
        if token_data.id != 1 && status != contest::ENDED {
            res.remove("details");
        }
        return Ok(Response::from(res).into_response());
    }
    Err(ErrorResponse::forbidden_with_str("无权查看"))
}

pub async fn find_csbmissions(
    Path(cid): Path<i64>,
    AxumJson(body): AxumJson<super::crud_service::FindBody>,
) -> ResponseResult {
    let res = csd::find(cid, body.filter.clone(), body.desc, body.limit, body.offset).await?;
    let cnt = csd::count(cid, body.filter).await?;
    Ok(Response::from(crate::json_map!("data"=> res, "count" => cnt)).into_response())
}

pub async fn get_rank_list(Path(cid): Path<i64>) -> ResponseResult {
    Ok(Response::from(cd::get_rank_list(cid).await?).into_response())
}



#[derive(Deserialize)]
pub struct NewPostBody {
    pub cid: i64,
    pub pid: i64,
    pub kind: i32,
    pub title: String,
    pub content: String
}
pub async fn new_post(token_data: Extension<UserToken>, AxumJson(body): AxumJson<NewPostBody>) -> ResponseResult {
    let uid = token_data.id;
    if body.cid != 0 && !crud::exists::<Contest>(body.cid).await.unwrap_or(false) {
        return Err(ErrorResponse::forbidden_with_str("not found"))
    }
    if body.pid != 0 && !crud::exists::<Problem>(body.pid).await.unwrap_or(false) {
        return Err(ErrorResponse::forbidden_with_str("not found"))
    }
    if body.cid != 0 && uid != 1 && allowed_enter_of_user(body.cid, uid).await.is_err() {
        return Err(ErrorResponse::forbidden_with_str("无权发表"))
    }
    let time = chrono::Local::now().timestamp_millis();
    let post = Post{
        id: 0,
        created_at: time,
        updated_at: time,
        uid,
        cid: body.cid,
        pid: body.pid,
        kind: body.kind,
        title: body.title,
        content: body.content,
        tags: vec![],
        comment_count: 0,
        comment_allowable: true
    };
    post_dao::insert(post).await?;
    Ok(Response::from_msg("ok").into_response())
}


#[derive(Deserialize)]
pub struct UpdatePostBody {
    pub post_id: i64,
    pub title: String,
    pub content: String
}

pub async fn update_post(token_data: Extension<UserToken>, AxumJson(body): AxumJson<UpdatePostBody>) -> ResponseResult {
    let o = crud::get_object::<Post>(body.post_id).await?;
    if token_data.id != 1 && o.uid != token_data.id {
        return Err(ErrorResponse::forbidden_with_str("没有权限"));
    }
    post_dao::update(body.post_id, json_map!("title" => body.title, "content" => body.content, "updated_at" => chrono::Local::now().timestamp_millis())).await?;
    Ok(Response::from_msg("ok").into_response())
}

pub async fn del_post(token_data: Extension<UserToken>, Path(post_id): Path<i64>) -> ResponseResult {
    let o = crud::get_object::<Post>(post_id).await?;
    if token_data.id != 1 && o.uid != token_data.id {
        return Err(ErrorResponse::forbidden_with_str("没有权限"));
    }
    post_dao::delete(post_id).await?;
    Ok(Response::from_msg("ok").into_response())
}

#[derive(Deserialize)]
pub struct CommentBody {
    pub post_id: i64,
    pub reply_id: i64,
    pub content: String
}
pub async fn comment(token_data: Extension<UserToken>, AxumJson(body): AxumJson<CommentBody>) -> ResponseResult {

    let cid = crud::get_one_value::<Post, i64>(body.post_id, "cid").await?;
    if cid != 0 && token_data.id != 1 && allowed_enter_of_user(cid, token_data.id).await.is_err() {
        return Err(ErrorResponse::forbidden_with_str("无权评论"))
    }

    let comment = Comment{
        id: 0,
        created_at: chrono::Local::now().timestamp_millis(),
        uid: token_data.id,
        post_id: body.post_id,
        reply_id: body.reply_id,
        content: body.content
    };

    post_dao::add_comment(comment).await?;
    Ok(Response::from_msg("ok").into_response())
}