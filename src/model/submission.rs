use super::traits::{Model, RedisSortedSet, StructFieldNames};
use crate::constants;
use crate::types::status as Status;
use rust_hoj::{FromJsonMap, FromPgRow, FromSql, GetFieldNames};
use serde::{Deserialize, Serialize};


#[derive(Serialize, Deserialize, Default, Debug, FromSql)]
pub struct CaseResult {
    pub id: i32,
    pub cpu_time: i32,
    pub memory: i32,
    pub info: String,
    pub status: i32,
}

#[derive(Serialize, Deserialize, Default, Debug, FromPgRow, GetFieldNames, FromJsonMap)]
pub struct Submission {
    pub id: i64,
    pub uid: i64,
    pub pid: i64,
    pub created_at: i64,

    pub lang: String,
    pub code: String,
    pub compile_info: String,     //编译信息
    pub case_count: i16,          //用例总数
    pub pass_count: i16,          //通过的数量
    pub length: i32,              //代码长度
    pub time: i32,                //用时
    pub memory: i32,              //内存消耗
    pub total_time: i32,          //所有样例总用时
    pub status: i32,              //结果
    pub error: String,            //错误信息
    pub details: Vec<CaseResult>, //各个用例的结果
    pub is_open: bool,
}

impl Model for Submission {
    fn table_name() -> &'static str {
        constants::SUBMISSION_TABLE_NAME
    }
    fn id(&self) -> i64 {
        self.id
    }
    fn expire() -> usize {
        8 * 3600
    }
}

impl Submission {
    pub fn from(uid: i64, pid: i64, lang: String, code: String) -> Self {
        let mut s = Submission::default();
        s.uid = uid;
        s.pid = pid;
        s.lang = lang;
        s.created_at = chrono::Local::now().timestamp_millis();
        s.length = code.len() as i32;
        s.code = code;
        s.status = Status::RUNNING;
        s
    }
}

impl RedisSortedSet for Submission {
    type M = Submission;
    fn zset_expire() -> usize {
        3600 * 24 * 3
    }
    fn zset_fields() -> &'static [&'static str] {
        &["id", "uid", "pid", "created_at", "status", "lang", "length", "time", "memory", "total_time", "is_open"]
    }
    fn one_dep_field() -> &'static str {
        "pid"
    }
}