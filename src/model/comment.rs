use super::traits::*;
use crate::constants;
use rust_hoj::{FromJsonMap, FromPgRow, GetFieldNames};
use serde::{Deserialize, Serialize};
#[derive(
Serialize, Deserialize, Debug, Clone, Default, FromPgRow, GetFieldNames, FromJsonMap,
)]
pub struct Comment {
    pub id: i64,
    pub created_at: i64,
    pub uid: i64,
    pub post_id: i64,
    pub reply_id: i64, //回复评论的id， 为0时为对post的评论
    pub content: String,
}
impl Model for Comment {
    fn table_name() -> &'static str {
        constants::COMMENT_TABLE_NAME
    }
    fn id(&self) -> i64 {
        self.id
    }
}

impl RedisSortedSet for Comment {
    type M = Comment;
    fn zset_expire() -> usize {
        24 * 3600
    }
    fn one_dep_field() -> &'static str {
        "post_id"
    }
}