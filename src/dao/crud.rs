use super::postgres;
use crate::dao::redis_db;
use crate::model::traits::*;
use crate::types::links::JsonMap;
use anyhow::Result;
use mobc_redis::redis::AsyncCommands;
use serde_json::{json, Value as Json};
use std::fmt::Display;
use tokio_postgres::types::Type;

pub fn json_to_sql(v: &Json) -> String {
    match v {
        Json::String(s) => format!("'{}'", s.replace("'", "''")),
        _ => format!("'{}'", v.to_string()),
    }
}

pub fn pg_row_to_json_map(row: tokio_postgres::Row) -> JsonMap {
    row.columns().into_iter().map(|c| {
        let value = match c.type_() {
            &Type::INT8 => json!(row.get::<&str, i64>(c.name())),
            &Type::INT4 => json!(row.get::<&str, i32>(c.name())),
            &Type::VARCHAR | &Type::TEXT => Json::String(row.get(c.name())),
            &Type::BOOL => Json::Bool(row.get(c.name())),
            &Type::INT2 => json!(row.get::<&str, i16>(c.name())),
            _ => redis_db::bytes_to_json(row.get(c.name()))
        };
        (c.name().to_string(), value)
    }).collect()
}

pub fn make_conditions_from_json_map(mp: JsonMap) -> String {
    if mp.len() == 0 {
        return "".into();
    }
    format!(
        "WHERE {}",
        mp.iter()
            .map(|(x, y)| format!("{} = {}", x, json_to_sql(y)))
            .reduce(|mut res, item| {
                res += " AND ";
                res += &item;
                res
            })
            .unwrap()
    )
}

pub async fn insert_by_map<T: Model>(info: &JsonMap) -> Result<i64> {
    let conn = postgres::get_pg_connect().await?;
    let (x, y) = info
        .iter()
        .filter(|(x, y)| x.as_str() != "id" && !y.is_null())
        .map(|(x, y)| (x.clone(), json_to_sql(y)))
        .reduce(|(mut rx, mut ry), (x, y)| {
            rx += ",";
            rx += &x;
            ry += ",";
            ry += &y;
            (rx, ry)
        })
        .unwrap();
    let sql = format!(
        "INSERT INTO \"{}\" ( {} ) VALUES ( {} ) RETURNING id",
        T::table_name(),
        x,
        y
    );
    let res = conn.query_one(sql.as_str(), &[]).await?;
    Ok(res.get("id"))
}

pub async fn insert<T: Model + RedisSortedSet, S: Display>(x: &T, dep: Option<S>) -> Result<i64> {
    if let Json::Object(mp) = json!(x) {
        let id = insert_by_map::<T>(&mp).await?;
        update_sorted_set::<T, S>(id, dep).await?;
        return Ok(id);
    }
    Err(anyhow::Error::msg("对象不能转换为 Json"))
}

pub async fn get_object_from_sql<T: Model>(id: i64) -> Result<T> {
    let conn = postgres::get_pg_connect().await?;
    let sql = format!("SELECT * FROM \"{}\" WHERE id = '{}'", T::table_name(), id);
    Ok(T::from(conn.query_one(sql.as_str(), &[]).await?))
}

pub async fn cache_object<T: Model>(id: i64) -> Result<T> {
    let o = get_object_from_sql::<T>(id).await?;
    let v = json!(o);
    redis_db::hset_map(T::redis_key(id), v.as_object().unwrap(), T::expire()).await?;
    Ok(o)
}

pub async fn get_object<T: Model>(id: i64) -> Result<T> {
    let mut conn = redis_db::get_conn().await?;
    let key = T::redis_key(id);
    if !conn.exists(key.as_str()).await? {
        return cache_object::<T>(id).await;
    }
    Ok(redis_db::hmget_map(key, T::field_names()).await?.into())
}

pub async fn exists<T: Model>(id: i64) -> Result<bool> {
    if !redis_db::exists(T::redis_key(id)).await? {
        if cache_object::<T>(id).await.is_err() {
            return Ok(false);
        }
    }
    Ok(true)
}

pub async fn count<T: Model>(conditions: JsonMap) -> Result<i64> {
    let sql = format!(
        "SELECT count(*) FROM \"{}\" {}",
        T::table_name(),
        make_conditions_from_json_map(conditions)
    );
    let conn = postgres::get_pg_connect().await?;
    Ok(conn.query_one(sql.as_str(), &[]).await?.get("count"))
}

pub async fn update_by_map_for_sql<T: Model>(id: i64, columns: &JsonMap) -> Result<()> {
    if columns.is_empty() {
        return Ok(());
    }
    let conn = postgres::get_pg_connect().await?;
    let items = columns
        .iter()
        .map(|(x, y)| format!("{} = {}", x, json_to_sql(y)))
        .reduce(|mut res, item| {
            res += ",";
            res += &item;
            res
        })
        .unwrap();
    let sql = format!(
        "UPDATE \"{}\" SET {} WHERE id = '{}'",
        T::table_name(),
        items,
        id
    );
    conn.execute(sql.as_str(), &[]).await?;
    if exists::<T>(id).await.unwrap_or(false) {
        redis_db::hset_map(T::redis_key(id), columns, T::expire())
            .await
            .unwrap_or(());
    }
    Ok(())
}

pub async fn update_by_map<T: Model + RedisSortedSet, S: Display>(id: i64, columns: &JsonMap, dep: Option<S>, ) -> Result<()> {
    if update_by_map_for_sql::<T>(id, columns).await.is_ok() {
        update_sorted_set::<T, S>(id, dep).await?;
    }
    Ok(())
}

pub async fn inc<T: Model + RedisSortedSet, S: Display>(id: i64, fields: &[&str], dep: Option<S>) -> Result<()> {
    let conn = postgres::get_pg_connect().await?;
    let mut sql = format!("UPDATE \"{}\" set  ", T::table_name());
    for i in 0..fields.len() {
        sql.push_str(
            format!(
                "{}={}+1 {}",
                fields[i],
                fields[i],
                match i + 1 < fields.len() {
                    true => ",",
                    false => " ",
                }
            )
                .as_str(),
        );
    }
    sql.push_str(format!("WHERE id = '{}'", id).as_str());
    if conn.execute(sql.as_str(), &[]).await? > 0 {
        redis_db::del(T::redis_key(id)).await.unwrap_or(());
        update_sorted_set::<T, S>(id, dep).await?;
    }
    Ok(())
}

// 修改操作
pub async fn insert_without_dev<T: Model + RedisSortedSet>(x: &T) -> Result<i64> {
    insert::<T, &str>(x, None).await
}

pub async fn update_by_map_without_dev<T: Model + RedisSortedSet>(id: i64, columns: &JsonMap) -> Result<()> {
    update_by_map::<T, &str>(id, columns, None).await
}

pub async fn inc_without_dev<T: Model + RedisSortedSet>(id: i64, fields: &[&str]) -> Result<()> {
    inc::<T, &str>(id, fields, None).await
}

pub async fn get_columns<T: Model>(id: i64, fields: &[&str]) -> Result<JsonMap> {
    if !exists::<T>(id).await? {
        return Err(anyhow::Error::msg("Not Found"));
    }
    redis_db::hmget_map(T::redis_key(id), fields).await
}

pub async fn get_values<T: Model>(id: i64, fields: &[&str]) -> Result<Vec<Json>> {
    if !exists::<T>(id).await? {
        return Err(anyhow::Error::msg("Not Found"));
    }
    redis_db::hmget_vec(T::redis_key(id), fields).await
}

pub async fn find_values_with_filter<T: Model>(fields: &[&str], filter: JsonMap) -> Result<Vec<tokio_postgres::Row>> {
    let conn = postgres::get_pg_connect().await?;
    let sql = format!(
        "SELECT {} FROM {} {} ORDER BY id ASC",
        fields.join(","),
        T::table_name(),
        make_conditions_from_json_map(filter)
    );
    let rows = conn.query(sql.as_str(), &[]).await?;
    Ok(rows)
}

pub async fn get_one_value<T: Model, F: serde::de::DeserializeOwned>( id: i64, field: &str) -> Result<F> {
    if !exists::<T>(id).await? {
        return Err(anyhow::Error::msg("Not Found"));
    }
    Ok(
        serde_json::from_value(redis_db::hget(T::redis_key(id), field).await?)
            .map_err(|e| anyhow::Error::msg(e.to_string()))?,
    )
}

pub async fn delete<T: Model + RedisSortedSet, S: Display>(id: i64, dep: Option<S>) -> Result<()> {
    let conn = postgres::get_pg_connect().await?;
    let sql = format!("DELETE FROM \"{}\" WHERE id = {}", T::table_name(), id);
    let _ = conn.query(sql.as_str(), &[]).await?;
    let key1 = T::redis_key(id);
    let key2 = match dep {
        None => T::zset_key(),
        Some(s) => T::zset_key_with_one_dep(&s)
    };
    redis_db::multi_cmd(vec![vec!["del", key1.as_str()], vec!["del", key2.as_str()]]).await?;
    Ok(())
}

pub async fn find_base_columns<T: Model + RedisSortedSet>( filter: Option<JsonMap>,  desc: Option<bool>, limit: Option<i32>, offset: Option<i32>) -> Result<Vec<JsonMap>> {

    let conn = postgres::get_pg_connect().await?;
    let sql = format!(
        "SELECT {} FROM \"{}\" {} ORDER BY id {} {} {}",
        T::zset_fields().join(","),
        T::table_name(),
        make_conditions_from_json_map(filter.unwrap_or(JsonMap::new())),
        match desc.unwrap_or(false) {
            true => "DESC",
            false => "ASC",
        },
        match limit {
            Some(x) => format!("LIMIT {}", x),
            None => format!(""),
        },
        match offset {
            Some(x) => format!("OFFSET {}", x),
            None => format!(""),
        }
    );
    Ok(conn
        .query(sql.as_str(), &[])
        .await
        .and_then(|rows| Ok(rows.into_iter().map(|r| pg_row_to_json_map(r)).collect()))?)
}

pub async fn get_id<T: Model>(conditions: JsonMap) -> Result<i64> {
    let sql = format!(
        "SELECT id FROM \"{}\" {} LIMIT 1",
        T::table_name(),
        make_conditions_from_json_map(conditions)
    );
    let conn = postgres::get_pg_connect().await?;
    Ok(conn.query_one(sql.as_str(), &[]).await?.get("id"))
}

pub async fn cache_with_zkey_and_sql<T: Model + RedisSortedSet>(zkey: &str, sql: &str) -> Result<()> {
    let conn = postgres::get_pg_connect().await?;
    let rows = conn.query(sql, &[]).await?;
    if rows.is_empty() {
        return Ok(());
    }
    let items: Vec<_> = rows
        .into_iter()
        .map(|row| {
            let mp: JsonMap = pg_row_to_json_map(row);
            (T::score_from_map(&mp), Json::Object(mp).to_string())
        })
        .collect();
    redis_db::zadd_multiple(zkey, items.as_slice()).await?;
    redis_db::expire(zkey, T::zset_expire()).await?;
    Ok(())
}

pub async fn cache_sorted_set<T: Model + RedisSortedSet, S: Display>(dep: Option<S>) -> Result<String> {
    let zkey = match &dep {
        None => T::zset_key(),
        Some(s) => T::zset_key_with_one_dep(s)
    };
    if !redis_db::exists(zkey.as_str()).await? {
        let sql = match &dep {
            None => T::sql(),
            Some(s) => T::sql_with_one_dep(s)
        };
        cache_with_zkey_and_sql::<T>(zkey.as_str(), sql.as_str()).await?;
    }
    Ok(zkey)
}

pub async fn update_sorted_set<T: Model + RedisSortedSet, S: Display>(id: i64, dep: Option<S>) -> Result<()> {
    let zkey = match &dep {
        None => T::zset_key(),
        Some(s) => T::zset_key_with_one_dep(s)
    };
    if redis_db::exists(zkey.as_str()).await? {
        let mp = get_columns::<T>(id, T::zset_fields()).await?;
        let score = T::score_from_map(&mp).to_string();
        let content = Json::Object(mp).to_string();
        return redis_db::multi_cmd(vec![
            vec![
                "ZREMRANGEBYSCORE",
                zkey.as_str(),
                score.as_str(),
                score.as_str(),
            ],
            vec!["ZADD", zkey.as_str(), score.as_str(), content.as_str()],
        ])
            .await;
    }
    Ok(())
}

pub async fn zcard<T: Model + RedisSortedSet, S: Display>(dep: Option<S>) -> Result<i64> {
    let zkey = cache_sorted_set::<T, S>(dep).await?;
    redis_db::zcard(zkey).await
}

pub async fn zrange<T: Model + RedisSortedSet, S: Display>(l: isize, r: isize, dep: Option<S>) -> Result<Vec<JsonMap>> {
    let zkey = cache_sorted_set::<T, S>(dep).await?;
    let x = redis_db::zrange(zkey.as_str(), l, r).await?;
    Ok(x.into_iter()
        .map(|item| serde_json::from_value(item).unwrap())
        .collect())
}

pub async fn zrevrange<T: Model + RedisSortedSet, S: Display>(l: isize, r: isize, dep: Option<S>) -> Result<Vec<JsonMap>> {
    let zkey = cache_sorted_set::<T, S>(dep).await?;
    let x = redis_db::zrevrange(zkey.as_str(), l, r).await?;
    Ok(x.into_iter()
        .map(|item| serde_json::from_value(item).unwrap())
        .collect())
}

pub async fn zrangebyscore<T: Model + RedisSortedSet, S: Display>( low: i64, high: i64, dep: Option<S>) -> Result<Vec<JsonMap>> {
    let zkey = cache_sorted_set::<T, S>(dep).await?;
    let x = redis_db::zrangebyscore(zkey, low, high).await?;
    Ok(x.into_iter()
        .map(|item| serde_json::from_str(item.as_str()).unwrap())
        .collect())
}

pub async fn zfind<T: Model + RedisSortedSet, S: Display>(desc: Option<bool>, limit: Option<i32>, offset: Option<i32>, dep: Option<S>) -> Result<Vec<JsonMap>> {
    let l = offset.unwrap_or(0) as isize;
    let r = limit.unwrap_or(0) as isize + l - 1;
    match desc.unwrap_or(false) {
        true => zrevrange::<T, S>(l, r, dep).await,
        false => zrange::<T, S>(l, r, dep).await,
    }
}

pub async fn base_find<T: Model + RedisSortedSet, S: Display>(filter: Option<JsonMap>, desc: Option<bool>, limit: Option<i32>, offset: Option<i32>, dep: Option<S>) -> Result<Vec<JsonMap>> {
    let filter = filter.unwrap_or(JsonMap::new());
    match filter.is_empty() {
        true => zfind::<T, S>(desc, limit, offset, dep).await,
        _ => find_base_columns::<T>(Some(filter), desc, limit, offset).await,
    }
}

pub async fn base_count<T: Model + RedisSortedSet, S: Display>( filter: Option<JsonMap>, dep: Option<S>) -> Result<i64> {
    let filter = filter.unwrap_or(JsonMap::new());
    match filter.is_empty() {
        true => zcard::<T, S>(dep).await,
        _ => count::<T>(filter).await,
    }
}

pub async fn cache_with_two_deps<T: Model + RedisSortedSet, S: Display, F:Display>(dep1:S, dep2:F) -> Result<String> {
    let zkey = T::zset_key_with_two_deps(&dep1, &dep2);
    if !redis_db::exists(zkey.as_str()).await? {
        let sql = T::sql_with_two_deps(&dep1, &dep2);
        cache_with_zkey_and_sql::<T>(zkey.as_str(), sql.as_str()).await?;
    }
    Ok(zkey)
}

pub async fn zfind_with_two_deps<T: Model + RedisSortedSet, S: Display, F:Display>(desc: Option<bool>, limit: Option<i32>, offset: Option<i32>, dep1:S, dep2: F) -> Result<Vec<JsonMap>> {
    let l = offset.unwrap_or(0) as isize;
    let r = limit.unwrap_or(0) as isize + l - 1;
    let zkey = cache_with_two_deps::<T, S, F>(dep1, dep2).await?;
    let res = match desc.unwrap_or(false) {
        true => redis_db::zrange(zkey.as_str(), l, r).await?,
        _ => redis_db::zrevrange(zkey.as_str(), l, r).await?
    };
    Ok(res.into_iter()
        .map(|item| serde_json::from_value(item).unwrap())
        .collect())
}

// mapper hashset key   a -> b  通常主键的映射，这些是不会修改的
const MAPPER_HSET: &str = "mapper_hset";

pub async fn add_mapper<K: AsRef<str>>(key: K, val: Json) -> Result<()> {
    redis_db::hset(MAPPER_HSET, key, &val).await
}



pub async fn del_mapper<K: AsRef<str>>(key: K) -> Result<()> {
    redis_db::hdel(MAPPER_HSET, &[key.as_ref()]).await
}

pub async fn get_mapper<K: AsRef<str>>(key: K) -> Result<Json> {
    redis_db::hget(MAPPER_HSET, key).await
}
