use super::traits::*;
use crate::constants;
use rust_hoj::{FromJsonMap, FromPgRow, FromSql, GetFieldNames};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default, FromSql)]
pub struct ContestProblem {
    pub pid: i64,
    pub index: String,
    pub title: String,
    pub label: String,
    pub first_solve_time: Option<i64>,
    pub accepted_count: Option<i32>,
    pub all_count: Option<i32>,
}

pub const PENDING: i16 = 0;
pub const RUNNING: i16 = 1;
pub const ENDED: i16 = 2;

#[derive(Serialize, Deserialize, Debug, Clone, Default, FromPgRow, GetFieldNames, FromJsonMap)]
pub struct Contest {
    pub id: i64,
    pub created_at: i64,
    pub title: String,
    pub begin: i64,  //比赛开始时间戳
    pub length: i32, //单位 分钟
    pub description: String,
    pub author: String,
    pub is_open: bool,
    pub password: String,
    pub format: String, // 赛制 ACM ,OI
    pub status: i16,    //Pending 0 ,Running 1 ,Ended 2
    pub team_count: i32,
    pub problems: Vec<ContestProblem>,
    pub clarifications: Vec<String>,
}

impl Model for Contest {
    fn table_name() -> &'static str {
        constants::CONTEST_TABLE_NAME
    }
    fn id(&self) -> i64 {
        self.id
    }
}

impl RedisSortedSet for Contest {
    type M = Contest;
}
