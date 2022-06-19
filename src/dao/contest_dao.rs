use super::submission_dao as sd;
use super::{crud, redis_db};
use crate::constants;
use crate::json_map;
use crate::model::{
    contest::{self, Contest, ContestProblem},
    csubmission::CSubmission,
    team::{ProblemStatus, Team},
};
use crate::types::links::JsonMap;
use crate::types::status as Status;
use crate::utils::judger::{acquire_judge_chance, judge, release_judge_chance};
use anyhow::Result;
use lazy_static::lazy_static;
use serde_json::{json, Value as Json};
use std::collections::{HashMap, HashSet};
use timer::{Guard, Timer};
use tokio::{
    runtime::Runtime,
    sync::mpsc::{unbounded_channel, UnboundedSender},
    sync::Mutex,
    sync::RwLock,
};

lazy_static! {
    pub static ref TIMER: Mutex<Timer> = Mutex::new(Timer::new());
    pub static ref CONTEST_GUARDS: Mutex<HashMap<i64, Guard>> = Mutex::new(HashMap::new());
    pub static ref UPDATE_CHANNEL: Mutex<HashMap<i64, UnboundedSender<i64>>> = Mutex::new(HashMap::new());
    pub static ref RUNTIME: Runtime = Runtime::new().expect("创建runtime 出错");
    pub static ref RANKLIST_TO_UPDATE: RwLock<HashSet<i64>> = RwLock::new(HashSet::new()); //是否存在新提交，有新提交需要更新榜单
}

async fn put_contest_of_pending(
    id: i64,
    dur_to_running: chrono::Duration,
    dur_to_end: chrono::Duration,
) -> Result<()> {
    println!("比赛 {} 等待开始", id);
    CONTEST_GUARDS.lock().await.remove(&id);

    // 更新比赛状态为Pending
    crud::update_by_map_without_dev::<Contest>(id, &json_map!("status" => contest::PENDING))
        .await?;

    CONTEST_GUARDS.lock().await.insert(
        id,
        TIMER
            .lock()
            .await
            .schedule_with_delay(dur_to_running, move || {
                RUNTIME.spawn(async move {
                    put_contest_of_running(id, dur_to_end).await.unwrap();
                });
            }),
    );
    Ok(())
}

async fn put_contest_of_running(id: i64, dur_to_end: chrono::Duration) -> Result<()> {
    println!("比赛 {} 进行中", id);
    CONTEST_GUARDS.lock().await.remove(&id);

    // 更新比赛状态为Running
    crud::update_by_map_without_dev::<Contest>(id, &json_map!("status" => contest::RUNNING))
        .await?;

    if !UPDATE_CHANNEL.lock().await.contains_key(&id) {
        // 创建更新提交的线程
        let (s, mut r) = unbounded_channel::<i64>();
        RUNTIME.spawn(async move {
            loop {
                let sid = r.recv().await.unwrap();
                if sid == 0 {
                    // println!("测评队列关闭");
                    break;
                }

                // 更新sid
                // println!("收到提交 {}", sid);
                let s = std::sync::Arc::new(crud::get_object::<CSubmission>(sid).await.unwrap());

                RANKLIST_TO_UPDATE.write().await.insert(s.cid);

                let s1 = s.clone();
                let h0 = tokio::spawn(async move {
                    update_team_of_submission(s).await.unwrap();
                });
                let h1 = tokio::spawn(async move {
                    update_cproblem_of_submission(s1).await.unwrap();
                });
                let _ = tokio::join!(h0, h1);
            }
        });
        UPDATE_CHANNEL.lock().await.insert(id, s);
    }

    CONTEST_GUARDS.lock().await.insert(
        id,
        TIMER.lock().await.schedule_with_delay(dur_to_end, move || {
            RUNTIME.spawn(async move {
                put_contest_of_ended(id).await.unwrap();
            });
        }),
    );

    Ok(())
}

async fn put_contest_of_ended(id: i64) -> Result<()> {
    println!("比赛 {} 已经结束", id);
    CONTEST_GUARDS.lock().await.remove(&id);

    // 更新比赛状态为Running
    crud::update_by_map_without_dev::<Contest>(id, &json_map!("status" => contest::ENDED)).await?;

    RUNTIME.spawn(async move {
        // 赛后10分钟删除测评队列,不再测评
        tokio::time::sleep(tokio::time::Duration::from_secs(
            constants::WAITING_TIME_CLOSE_JUDGING_QUEUE,
        ))
        .await;
        if let Some(r) = UPDATE_CHANNEL.lock().await.remove(&id) {
            r.send(0).unwrap();
        }
        RANKLIST_TO_UPDATE.write().await.remove(&id);
        redis_db::del(get_run_id_key(id)).await.unwrap();
    });
    Ok(())
}

async fn put_contest(id: i64, begin: i64, length: i32) -> Result<()> {
    let end = begin + length as i64 * 60000;
    let now = chrono::Local::now().timestamp_millis();
    if begin > now {
        //Pending
        put_contest_of_pending(
            id,
            chrono::Duration::milliseconds(begin - now),
            chrono::Duration::milliseconds(end - begin),
        )
        .await?;
    } else if end > now {
        // Running
        put_contest_of_running(id, chrono::Duration::milliseconds(end - now)).await?;
    } else {
        // Ended
        put_contest_of_ended(id).await?;
    }
    Ok(())
}

pub async fn fresh() -> Result<()> {
    let cs = crud::zrevrange::<Contest, i64>(0, -1, None).await?;
    for c in cs.into_iter() {
        let status= c.get("status").unwrap().as_i64().unwrap_or(contest::ENDED as i64);
        if status != contest::ENDED as i64 {
            put_contest(c.get("id").unwrap().as_i64().unwrap(),
                        c.get("begin").unwrap().as_i64().unwrap(),
                        c.get("length").unwrap().as_i64().unwrap() as i32).await?;
        }
    }
    Ok(())
}

fn make_up_problems(problems: &mut Vec<ContestProblem>) {
    problems.iter_mut().for_each(|p| {
        if p.accepted_count.is_none() {
            p.accepted_count = Some(0)
        }
        if p.all_count.is_none() {
            p.all_count = Some(0)
        }
        if p.first_solve_time.is_none() {
            p.first_solve_time = Some(i64::MAX)
        }
    });
}
pub async fn update(id: i64, mut mp: JsonMap) -> Result<()> {
    if mp.contains_key("problems") {
        let mut problems: Vec<ContestProblem> =
            serde_json::from_value(mp.remove("problems").unwrap()).unwrap();
        make_up_problems(&mut problems);
        mp.insert("problems".into(), json!(problems));
    }
    crud::update_by_map_without_dev::<Contest>(id, &mp).await?;
    if mp.contains_key("length") || mp.contains_key("begin") {
        let v = crud::get_values::<Contest>(id, &["begin", "length"]).await?;
        put_contest(id, v[0].as_i64().unwrap(), v[1].as_i64().unwrap() as i32).await?;
    }
    Ok(())
}

pub async fn insert(mut contest: Contest) -> Result<i64> {
    make_up_problems(&mut contest.problems);
    contest.created_at = chrono::Local::now().timestamp_millis();

    let id = crud::insert_without_dev(&contest).await?;
    put_contest(id, contest.begin, contest.length).await?;
    Ok(id)
}

pub async fn find(
    filter: Option<JsonMap>,
    desc: Option<bool>,
    limit: Option<i32>,
    offset: Option<i32>,
) -> Result<Vec<JsonMap>> {
    crud::base_find::<Contest, i64>(filter, desc, limit, offset, None).await
}

pub async fn count(filter: Option<JsonMap>) -> Result<i64> {
    crud::base_count::<Contest, i64>(filter, None).await
}

pub async fn get_fields(id: i64, wants: Vec<&str>) -> Result<JsonMap> {
    crud::get_columns::<Contest>(id, wants.as_slice()).await
}

pub async fn update_team_of_submission(s: std::sync::Arc<CSubmission>) -> Result<()> {
    // println!("team updated");
    let t = crud::get_object::<Team>(s.tid).await?;
    let mut result: HashMap<String, ProblemStatus> =
        serde_json::from_str(t.result.as_str()).unwrap();

    let mut ps = result.remove(s.label.as_str()).unwrap_or(ProblemStatus {
        label: s.label.clone(),
        fail_times: 0,
        pass_time: None,
        score: 0,
    });

    if ps.pass_time.is_some() && ps.pass_time.unwrap() <= s.created_at.unwrap() {
        return Ok(());
    }

    match s.status {
        Status::AC => {
            let rows = crud::find_values_with_filter::<CSubmission>(
                &["status", "created_at"],
                json_map!("cid" => s.cid, "tid" => t.id, "pid" => s.pid),
            )
            .await?;
            ps.fail_times = 0;
            ps.score = 100;
            for row in rows.into_iter() {
                let status: i32 = row.get("status");
                if status == Status::QUEUEING || status == Status::RUNNING {
                    continue;
                }
                if status == Status::AC {
                    ps.pass_time = Some(row.get("created_at"));
                    break;
                } else {
                    ps.fail_times += 1;
                }
            }
        }
        _ => {
            ps.fail_times += 1;
            ps.score = std::cmp::max(
                ps.score,
                (s.pass_count as f64 / s.case_count as f64 * 100.0) as i32,
            )
        }
    }

    result.insert(s.label.clone(), ps);

    let update_map = crate::json_map!("result" => json!(result).to_string());
    crud::update_by_map::<Team, i64>(t.id, &update_map, Some(t.cid)).await
}

pub async fn update_cproblem_of_submission(s: std::sync::Arc<CSubmission>) -> Result<()> {
    let mut problems =
        crud::get_one_value::<Contest, Vec<ContestProblem>>(s.cid, "problems").await?;
    for item in problems.iter_mut() {
        if item.pid == s.pid {
            item.all_count = Some(item.all_count.unwrap_or(0) + 1);
            if s.status == Status::AC {
                if item.first_solve_time.is_none()
                    || item.first_solve_time.unwrap() > s.created_at.unwrap()
                {
                    item.first_solve_time = s.created_at;
                }
                item.accepted_count = Some(item.accepted_count.unwrap_or(0) + 1);
            }
            break;
        }
    }
    let update_map = crate::json_map!("problems" => problems);
    crud::update_by_map_without_dev::<Contest>(s.cid, &update_map).await
}

pub fn team_id_map_key(cid: i64, uid: i64) -> String {
    format!("tid_cid:{}_uid:{}", cid, uid)
}
pub async fn get_team_id(cid: i64, uid: i64) -> Result<i64> {
    let key = team_id_map_key(cid, uid);
    let x = redis_db::get(key.as_str()).await?;
    if x.is_null() {
        let rows =
            crud::find_values_with_filter::<Team>(&["id"], json_map!("cid" => cid, "uid" => uid))
                .await?;
        if rows.is_empty() {
            return Err(anyhow::Error::msg("not found"));
        }
        let id: i64 = rows[0].get("id");
        redis_db::set(key, &json!(id), 3600 * 8).await?;
        return Ok(id);
    }
    Ok(x.as_i64().unwrap())
}

pub async fn register_team(cid: i64, uid: i64, username: String, password: &str) -> Result<Team> {
    let t = get_team_id(cid, uid).await;
    if t.is_ok() {
        return Err(anyhow::Error::msg("已注册"));
    }
    if uid != 1 {
        let values = crud::get_values::<Contest>(cid, &["status", "is_open", "password"]).await?;

        if values[0].as_i64().unwrap() == 2 {
            return Err(anyhow::Error::msg("比赛已结束"));
        }

        if !values[1].as_bool().unwrap() && values[2].as_str().unwrap() != password {
            return Err(anyhow::Error::msg("密码错误"));
        }
    }

    //注册
    let mut t = Team::default();
    t.cid = cid;
    t.uid = uid;
    t.name = username;
    t.id = crud::insert::<Team, i64>(&t, Some(t.cid)).await?;
    tokio::spawn(async move {
        crud::inc_without_dev::<Contest>(cid, &["team_count"])
            .await
            .unwrap();
        RANKLIST_TO_UPDATE.write().await.insert(cid);
    });
    Ok(t)
}

//允许用户进入表示， 1.用户是超级管理员 2.比赛是开放的 3.用户已经注册比赛
pub async fn allowed_enter_of_user(cid: i64, uid: i64) -> Result<()> {
    let is_open = crud::get_one_value::<Contest, bool>(cid, "is_open").await?;
    if uid == 1 {
        return Ok(());
    }
    if is_open {
        return Ok(());
    }
    let _ = get_team_id(cid, uid).await?;
    Ok(())
}

const STATUS_KEY_EXPIRE: usize = 120;
pub fn csm_status_key(sid: i64) -> String {
    format!("status_of_csm:{}", sid)
}
pub async fn get_csm_status(sid: i64) -> Result<i32> {
    let key = csm_status_key(sid);
    let res = redis_db::get(key.as_str()).await?;
    if res.is_null() {
        crud::get_one_value::<CSubmission, i32>(sid, "status").await
    } else {
        Ok(res.as_i64().unwrap() as i32)
    }
}

fn get_run_id_key(cid: i64) -> String {
    format!("contest_run_id:{}", cid)
}
pub async fn handle_contest_submission(
    tid: i64,
    cid: i64,
    cp: ContestProblem,
    lang: String,
    code: String,
    test_all: bool,
) -> Result<i64> {
    let mut s = CSubmission::from(tid, cid, cp.pid, cp.label.clone(), lang, code);
    s.run_id = redis_db::incr(get_run_id_key(cid)).await?;
    s.author = crud::get_one_value::<Team, String>(tid, "name").await?;
    s.id = crud::insert::<CSubmission, i64>(&s, Some(cid)).await?;
    let sid = s.id;
    tokio::spawn(async move {
        let csm_status_key = csm_status_key(s.id);
        redis_db::set(
            csm_status_key.as_str(),
            &json!(Status::QUEUEING),
            STATUS_KEY_EXPIRE,
        )
        .await
        .unwrap();
        acquire_judge_chance().await;

        redis_db::set(
            csm_status_key.as_str(),
            &json!(Status::RUNNING),
            STATUS_KEY_EXPIRE,
        )
        .await
        .unwrap();
        let jc = sd::make_judge_config(s.pid, s.lang.clone(), s.code, test_all)
            .await
            .unwrap();

        //代码运行
        let res = judge(jc).await.unwrap();
        release_judge_chance().await;

        //更新提交
        let update_map = sd::make_update_map(&res);
        redis_db::set(
            csm_status_key.as_str(),
            &json!(res.status),
            STATUS_KEY_EXPIRE,
        )
        .await
        .unwrap();
        crud::update_by_map::<CSubmission, i64>(s.id, &update_map, Some(s.cid))
            .await
            .unwrap();

        if let Some(x) = UPDATE_CHANNEL.lock().await.get(&s.cid) {
            x.send(s.id).unwrap();
        }
    });

    Ok(sid)
}

fn rank_list_key(cid: i64) -> String {
    format!("ranklist:{}", cid)
}
fn ranklist_expire() -> usize {
    24 * 3600 * 3
}
async fn cache_rank_list(cid: i64) -> Result<Json> {
    let cid2 = cid;
    let h0 = tokio::spawn(async move {
        let mut problems = crud::get_one_value::<Contest, Vec<ContestProblem>>(cid2, "problems")
            .await
            .unwrap();
        make_up_problems(&mut problems);
        problems
    });
    let cid3 = cid;
    let h1 = tokio::spawn(async move {
        let mut vmp = crud::zrange::<Team, i64>(0, -1, Some(cid3))
            .await
            .unwrap();
        vmp.iter_mut()
        .for_each(|mp|{
            let x = mp.remove("result").unwrap();
            mp.insert("result".into(), serde_json::from_str(x.as_str().unwrap()).unwrap());
        });
        vmp
    });

    let (x, y) = tokio::join!(h0, h1);
    let js = json!({"problems":x?, "teams":y?});

    redis_db::set(rank_list_key(cid), &js, ranklist_expire()).await?;
    Ok(js)
}

pub async fn get_rank_list(cid: i64) -> Result<Json> {
    let key = rank_list_key(cid);
    if !redis_db::exists(key.as_str()).await? {
        return cache_rank_list(cid).await;
    }
    if RANKLIST_TO_UPDATE.read().await.contains(&cid) {
        let mut st = RANKLIST_TO_UPDATE.write().await;
        if st.contains(&cid) {
            let x = cache_rank_list(cid).await?;
            st.remove(&cid);
            return Ok(x);
        }
    }
    redis_db::get(key).await
}
