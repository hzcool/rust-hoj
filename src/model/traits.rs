use crate::types::links::JsonMap;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::convert::From;
use std::fmt::Display;

pub trait StructFieldNames {
    fn field_names() -> &'static [&'static str];
}

pub trait Model:
Serialize
+ DeserializeOwned
+ Default
+ From<tokio_postgres::Row>
+ StructFieldNames
+ From<JsonMap>
{
    fn expire() -> usize {
        24 * 3600 * 3
    }
    fn table_name() -> &'static str;
    fn id(&self) -> i64;
    fn redis_key(id: i64) -> String {
        format!("{}:{}", Self::table_name(), id)
    }
}

// 维护redis zset， 方便区间查找
pub trait RedisSortedSet
{
    type M: Model;
    fn zset_expire() -> usize {
        0
    }

    fn zset_fields() -> &'static [&'static str] {
        Self::M::field_names()
    }

    //redis维护与字段无关
    fn zset_key() -> String {
        format!("{}_zset", Self::M::table_name())
    }

    fn sql() -> String {
        format!("SELECT {} FROM \"{}\"", Self::zset_fields().join(","), Self::M::table_name())
    }

    fn one_dep_field() -> &'static str {
        ""
    }

    fn zset_key_with_one_dep<S: Display>(dep: &S) -> String {
        format!(
            "{}_zset:{}={}",
            Self::M::table_name(),
            Self::one_dep_field(),
            dep
        )
    }

    fn sql_with_one_dep<S: Display>(dep: &S) -> String {
        format!(
            "SELECT {} FROM \"{}\" WHERE {} = '{}'",
            Self::zset_fields().join(","),
            Self::M::table_name(),
            Self::one_dep_field(),
            dep
        )
    }

    fn two_dep_fields() -> (&'static str, &'static str) {
        ("", "")
    }

    fn zset_key_with_two_deps<S: Display, T:Display>(dep1: &S, dep2: &T) -> String {
        let (f1, f2) = Self::two_dep_fields();
        format!(
            "{}_zset:{}={},{}={}",
            Self::M::table_name(),
            f1,
            dep1,
            f2,
            dep2
        )
    }

    fn sql_with_two_deps<S: Display, T:Display>(dep1: &S, dep2: &T) -> String {
        let (f1, f2) = Self::two_dep_fields();
        format!(
            "SELECT {} FROM \"{}\" WHERE {}='{}' AND {}='{}'",
            Self::zset_fields().join(","),
            Self::M::table_name(),
            f1,
            dep1,
            f2,
            dep2
        )
    }

    fn score_from_map(mp: &JsonMap) -> i64 {
        mp.get("id").unwrap().as_i64().unwrap()
    }
}
