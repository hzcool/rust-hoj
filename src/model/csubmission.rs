use super::submission::CaseResult;
use super::traits::*;
use crate::constants;
use crate::types::status as Status;
use serde::{Deserialize, Serialize};

use rust_hoj::{FromJsonMap, FromPgRow, GetFieldNames};


#[derive(Serialize, Deserialize, Default, Debug, FromPgRow, GetFieldNames, FromJsonMap)]
pub struct CSubmission {
    pub id: i64,
    pub run_id: i64,
    pub tid: i64,
    pub cid: i64,
    pub pid: i64,
    pub label: String,
    pub author: String,
    pub created_at: Option<i64>,

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
impl CSubmission {
    pub fn from(tid: i64, cid: i64, pid: i64, label: String, lang: String, code: String) -> Self {
        let mut s = Self::default();
        s.created_at = Some(chrono::Local::now().timestamp_millis());
        s.tid = tid;
        s.cid = cid;
        s.pid = pid;
        s.label = label;
        s.lang = lang;
        s.length = code.len() as i32;
        s.code = code;
        s.status = Status::RUNNING;
        s
    }
}

impl Model for CSubmission {
    // type B = BaseCSubmission;
    fn table_name() -> &'static str {
        constants::CSUBMISSION_TABLE_NAME
    }
    fn id(&self) -> i64 {
        self.id
    }
    fn expire() -> usize {
        8 * 3600
    }
}

impl RedisSortedSet for CSubmission {
    type M = CSubmission;
    fn zset_expire() -> usize {
        3600 * 8
    }
    fn zset_fields() -> &'static [&'static str] {
        &["id", "run_id", "tid", "cid", "pid", "label", "author", "created_at", "status", "lang", "length", "time", "memory", "total_time", "is_open"]
    }
    fn one_dep_field() -> &'static str {
        "cid"
    }
}

// #[derive(
//     Serialize, Deserialize, Default, Debug, FromPgRow, GetFieldNames, FromJsonMap, IntoJsonMap,
// )]
// pub struct BaseCSubmission {
//     pub id: i64,
//     pub run_id: i64,
//     pub tid: i64,
//     pub cid: i64,
//     pub pid: i64,
//     pub label: String,
//     pub author: String,
//     pub created_at: i64,
//     pub status: i32,
//     pub lang: String,
//     pub length: i32,
//     pub time: i32,       //用时
//     pub memory: i32,     //内存消耗
//     pub total_time: i32, //所有样例总用时
//     pub is_open: bool,
// }
//
// impl SortedSetBaseModel for BaseCSubmission {
//     type M = CSubmission;
//     fn dev_field() -> &'static str {
//         "cid"
//     }
//     fn expire() -> usize {
//         3600 * 8
//     }
// }
