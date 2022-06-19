use super::traits::*;
use crate::constants;
use rust_hoj::{FromJsonMap, FromPgRow, GetFieldNames, IntoJsonMap};
use serde::{Deserialize, Serialize};


#[derive(Serialize, Deserialize, Debug, Clone, Default, IntoJsonMap, FromJsonMap)]
pub struct ProblemStatus {
    pub label: String,
    pub fail_times: i32,
    pub pass_time: Option<i64>, //通过该题的时间戳
    pub score: i32,
}

#[derive(
    Serialize, Deserialize, Debug, Clone, FromPgRow, GetFieldNames, FromJsonMap, IntoJsonMap,
)]
pub struct Team {
    pub id: i64,
    pub cid: i64,
    pub uid: i64,
    pub name: String,   //队伍名
    pub result: String, // hashmap(label: ps)
}

impl Default for Team {
    fn default() -> Self {
        Team {
            id: 0,
            cid: 0,
            uid: 0,
            name: "".into(),
            result: "{}".into(),
        }
    }
}

impl Model for Team {
    fn table_name() -> &'static str {
        constants::TEAM_TABLE_NAME
    }
    fn id(&self) -> i64 {
        self.id
    }
}

impl RedisSortedSet for Team {
    type M = Team;
    fn zset_expire() -> usize {
        3600 * 24 * 3
    }
    fn one_dep_field() -> &'static str {
        "cid"
    }
}
