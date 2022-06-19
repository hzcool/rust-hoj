use super::traits::*;
use crate::constants;
use rust_hoj::{FromJsonMap, FromPgRow, GetFieldNames};
use serde::{Deserialize, Serialize};
use super::problem::Tag;



#[derive(
    Serialize, Deserialize, Debug, Clone, Default, FromPgRow, GetFieldNames, FromJsonMap,
)]
pub struct Post {
    pub id: i64,
    pub created_at: i64,
    pub updated_at: i64,

    pub uid: i64,  // 作者id
    pub cid: i64,  // 比赛id,  为0时不是比赛题目
    pub pid: i64,  // 题目id

    pub kind: i32, //0discuss  1solution
    pub title: String,
    pub content: String,
    pub tags: Vec<Tag>,

    pub comment_count: u32,
    pub comment_allowable: bool
}

impl Model for Post {
    fn table_name() -> &'static str {
        constants::POST_TABLE_NAME
    }
    fn id(&self) -> i64 {
        self.id
    }
}

impl RedisSortedSet for Post {
    type M = Post;
    fn zset_expire() -> usize {
        24 * 3600
    }
    fn two_dep_fields() -> (&'static str, &'static str) {
        ("cid", "pid")
    }
}







