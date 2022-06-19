use super::traits::{Model, RedisSortedSet};
use crate::model::traits::StructFieldNames;
use crate::types::links::JsonMap;
use rust_hoj::{FromJsonMap, FromPgRow, FromSql, GetFieldNames};
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Serialize, Deserialize, Debug, Clone, Default, FromSql)]
pub struct SpjConfig {
    pub spj_lang: String, //特判语言，
    pub spj_src: String,  //特判代码
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, FromSql)]
pub struct TestCase {
    pub id: i32,
    pub input_name: String, //测试名
    pub input_size: i32,
    pub output_name: String,
    pub output_size: i32,
    pub max_output_size: Option<i32>, //测评时的最大输出大小
}

#[derive(Serialize, Deserialize, Debug, Clone, FromSql)]
pub struct Checker {
    typ: String,
    mode: Option<i32>,
    epsilon: Option<f64>,
}
impl Default for Checker {
    fn default() -> Self {
        Self {
            typ: "standard".to_string(),
            mode: None,
            epsilon: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, FromSql)]
pub struct Tag {
    label: String,
    color: String,
}

#[derive(
Validate, Serialize, Deserialize, Debug, Clone, Default, FromPgRow, GetFieldNames, FromJsonMap,
)]
pub struct Problem {
    pub id: i64,
    pub index: String,
    pub is_open: bool,
    pub created_at: i64,
    pub author: Option<String>,
    pub source: Option<String>,
    pub title: String,
    pub background: String,
    pub statement: String,
    pub input: String,
    pub output: String,
    pub hint: String,
    pub examples_in: Vec<String>,
    pub examples_out: Vec<String>,

    #[validate(range(min = 200, message = "最小时间限制 200 ms"))]
    #[validate(range(max = 60000, message = "最大时间限制 60000 ms "))]
    pub time_limit: i32, //毫秒

    #[validate(range(min = 20, message = "最小空间限制 20 mb"))]
    #[validate(range(max = 2048, message = "最大空间限制 2048 mb "))]
    pub memory_limit: i32, //mb

    pub tags: Vec<Tag>,
    pub accepted_count: i32,
    pub all_count: i32,

    pub spj_config: Option<SpjConfig>,
    pub test_cases: Vec<TestCase>,
    pub checker: Checker,
}

impl Model for Problem {
    fn table_name() -> &'static str {
        crate::constants::PROBLEM_TABLE_NAME
    }
    fn id(&self) -> i64 {
        self.id
    }
}

impl RedisSortedSet for Problem {
    type M = Problem;
    fn zset_fields() -> &'static [&'static str] {
        &["id", "index", "is_open", "created_at", "author", "source", "title", "tags", "accepted_count", "all_count"]
    }

    fn one_dep_field() -> &'static str {
        "is_open"
    }

    fn score_from_map(mp: &JsonMap) -> i64 {
        let index = mp.get("index").unwrap().as_str().unwrap();
        index[1..].parse::<i64>().unwrap()
    }
}
