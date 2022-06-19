use super::traits::{Model, RedisSortedSet, StructFieldNames};
use crate::constants;
use crate::types::links::JsonMap;
use rust_hoj::{FromJsonMap, FromPgRow, GetFieldNames, IntoJsonMap};
use serde::{Deserialize, Serialize};

#[derive(
Serialize,
Deserialize,
Default,
Debug,
Clone,
FromPgRow,
GetFieldNames,
FromJsonMap,
IntoJsonMap,
)]
pub struct User {
    pub id: i64,
    pub created_at: Option<i64>,
    pub username: String,
    pub password: String,
    pub school: String,
    pub email: String,
    pub role: Option<String>, //user, admin, super_admin
    pub description: Option<String>,
    pub avatar: String,
    pub rating: Option<i32>,
    pub privilege: i64,
    pub solved: i32,
    pub all_count: i32,
    pub accepted_count: i32,
    pub solved_problems: Vec<i64>,
    pub failed_problems: Vec<i64>,
}

impl User {
    pub fn jwt_token_key(id: i64) -> String {
        format!("jwt_token_of_user:{}", id)
    }
    pub fn redis_score(id: i64, solved: i64, accepted_count: i64, all_count: i64) -> i64 {
        let ratio: i64 = match all_count {
            0 => 1000000000,
            _ => ((accepted_count as f64 / all_count as f64) * 1000000000.0) as i64,
        };
        solved * 10000000000 + ratio + id
    }
}

impl Model for User {
    fn table_name() -> &'static str {
        constants::USER_TABLE_NAME
    }
    fn id(&self) -> i64 {
        self.id
    }
}

impl RedisSortedSet for User {
    type M = User;
    fn zset_fields() -> &'static [&'static str] {
        &["id", "username", "role", "avatar", "school", "solved", "all_count", "accepted_count"]
    }
    fn score_from_map(mp: &JsonMap) -> i64 {
        let id = mp.get("id").unwrap().as_i64().unwrap();
        let solved = mp.get("solved").unwrap().as_i64().unwrap();
        let all_count = mp.get("all_count").unwrap().as_i64().unwrap();
        let accepted_count = mp.get("accepted_count").unwrap().as_i64().unwrap();
        let ratio: i64 = match all_count {
            0 => 1000000000,
            _ => ((accepted_count as f64 / all_count as f64) * 1000000000.0) as i64,
        };
        solved * 10000000000 + ratio + id
    }
}
