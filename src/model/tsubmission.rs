use super::traits::*;
use crate::constants;
use rust_hoj::{FromJsonMap, FromPgRow, GetFieldNames};
use serde::{Deserialize, Serialize};



#[derive(Serialize, Deserialize, Default, Debug, FromPgRow, GetFieldNames, FromJsonMap)]
pub struct TSubmission {
    pub id: i64,
    pub uid: i64,
    pub pid: i64,
    pub created_at: i64,

    pub lang: String,
    pub code: String,
    pub length: i32,
    pub result: String, //json结果
}

impl Model for TSubmission {
    fn table_name() -> &'static str {
        constants::TSUBMISSION_TABLE_NAME
    }
    fn id(&self) -> i64 {
        self.id
    }
    fn expire() -> usize {
        8 * 3600
    }
}

impl TSubmission {
    pub fn from(uid: i64, pid: i64, lang: String, code: String) -> Self {
        let mut s = TSubmission::default();
        s.uid = uid;
        s.pid = pid;
        s.lang = lang;
        s.created_at = chrono::Local::now().timestamp_millis();
        s.length = code.len() as i32;
        s.code = code;
        s
    }
}

impl RedisSortedSet for TSubmission {
    type M = TSubmission;
    fn zset_expire() -> usize {
        3600 * 24
    }
    fn zset_fields() -> &'static [&'static str] {
        &["id", "uid", "pid", "created_at", "length", "lang", "code", "result"]
    }
    fn one_dep_field() -> &'static str {
        "pid"
    }
}
