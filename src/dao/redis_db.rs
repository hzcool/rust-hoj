use std::collections::BTreeMap;
use crate::constants;
use crate::types::links::JsonMap;
use mobc_redis::{
    mobc::Pool,
    RedisConnectionManager,
    redis,
    redis::{AsyncCommands, aio::Connection, Client, ToRedisArgs}
};
use anyhow::Result;
use serde_json::{Map, Value as Json};

use lazy_static::lazy_static;
lazy_static! {
    static ref POOL: Pool<RedisConnectionManager> = create_redis_pool();
}

fn create_redis_pool() -> Pool<RedisConnectionManager> {
    let client =
        Client::open(crate::config::env::redis_url()).expect("无法连接到 redis 数据库");
    let manager = RedisConnectionManager::new(client);
    Pool::builder()
        .max_open(constants::REDIS_POOL_SIZE)
        .build(manager)
}


pub async fn get_conn() -> Result<mobc_redis::mobc::Connection<RedisConnectionManager>> {
    Ok(POOL.get().await.expect(" 获取 redis 连接失败"))
}

pub async fn ping() {
    let mut conn = get_conn().await.expect("redis连接失败");
    let res: String = redis::cmd("PING")
        .query_async(&mut conn as &mut Connection)
        .await
        .unwrap();
    println!("redis : {}", res);
}


pub fn bytes_to_json(bytes: Vec<u8>) -> Json {
    match bytes.len() {
        0 => Json::Null,
        _ => serde_json::from_slice(bytes.as_slice()).unwrap()
    }
}

pub fn bytes_vec_to_json_vec(v: Vec<Vec<u8>>) -> Vec<Json> {
    v.into_iter().map(|bytes| bytes_to_json(bytes)).collect()
}

pub fn bytes_vec_to_map(v: Vec<Vec<u8>>) -> JsonMap {
    let mut iter = v.into_iter();
    let mut mp: Map<String, Json> = Map::new();
    loop {
        match iter.next() {
            Some(bytes) => {
                mp.insert(
                    String::from_utf8(bytes).unwrap(),
                    bytes_to_json(iter.next().unwrap()),
                );
            }
            None => break,
        }
    }
    mp
}

pub fn bytes_vec_to_json_map_vec(v: Vec<Vec<u8>>) -> Vec<JsonMap> {
    v.into_iter()
        .map(|bytes| serde_json::from_slice(bytes.as_slice()).unwrap())
        .collect()
}


pub async fn set<K: AsRef<str>>(key: K, val: &Json, expire: usize) -> Result<()> {
    let mut conn = get_conn().await?;
    let mut cmd = redis::cmd("set");
    cmd.arg(key.as_ref()).arg(val.to_string());
    if expire > 0 {
        cmd.arg("EX").arg(expire);
    }
    cmd.query_async(&mut conn as &mut Connection).await?;
    Ok(())
}

pub async fn get<K: AsRef<str>>(key: K) -> Result<Json> {
    let mut conn = get_conn().await?;
    let bytes: Vec<u8> = conn.get(key.as_ref()).await?;
    Ok(bytes_to_json(bytes))
}

pub async fn expire<K: AsRef<str>>(key: K, expire: usize) -> Result<()> {
    if expire == 0 {
        return Ok(());
    }
    let mut conn = get_conn().await?;
    Ok(conn.expire(key.as_ref(), expire).await?)
}

pub async fn exists<K: AsRef<str>>(key: K) -> Result<bool> {
    let mut conn = get_conn().await?;
    Ok(conn.exists(key.as_ref()).await?)
}

pub async fn del<K: AsRef<str>>(key: K) -> Result<()> {
    let mut conn = get_conn().await?;
    let _ = conn.del(key.as_ref()).await?;
    Ok(())
}

pub async fn incr<K: AsRef<str>>(key: K) -> Result<i64> {
    let mut conn = get_conn().await?;
    Ok(conn.incr(key.as_ref(), 1).await?)
}

pub async fn hexists<K: AsRef<str>, M: ToRedisArgs + Send + Sync>(
    key: K,
    field: M,
) -> Result<bool> {
    let mut conn = get_conn().await?;
    Ok(conn.hexists(key.as_ref(), field).await?)
}

pub async fn hset<K: AsRef<str>, F: AsRef<str>>(key: K, field: F, value: &Json) -> Result<()> {
    let mut conn = get_conn().await?;
    Ok(conn
        .hset(key.as_ref(), field.as_ref(), value.to_string())
        .await?)
}

//expire 为 0 标识不设置过期时间
pub async fn hset_map<K: AsRef<str>>(key: K, mp: &JsonMap, expire: usize) -> Result<()> {
    let mut conn = get_conn().await?;
    let items: Vec<_> = mp.iter().map(|(x, y)| (x, y.to_string())).collect();
    let _: () = conn.hset_multiple(key.as_ref(), items.as_ref()).await?;
    if expire > 0 {
        let _: () = conn.expire(key.as_ref(), expire).await?;
    }
    Ok(())
}

pub async fn hget<K: AsRef<str>, F: AsRef<str>>(key: K, field: F) -> Result<Json> {
    let mut conn = get_conn().await?;
    let bytes: Vec<u8> = conn.hget(key.as_ref(), field.as_ref()).await?;
    Ok(bytes_to_json(bytes))
}

pub async fn hdel<K: AsRef<str>>(key: K, fields: &[&str]) -> Result<()> {
    let mut conn = get_conn().await?;
    Ok(conn.hdel(key.as_ref(), fields).await?)
}

pub async fn hmget_map<K: AsRef<str>>(key: K, fields: &[&str]) -> Result<JsonMap> {
    if fields.len() == 0 {
        return Ok(JsonMap::new());
    }
    if fields.len() == 1 {
        let mp = crate::json_map!(fields[0].to_string() => hget(key, fields[0]).await?);
        return Ok(mp);
    }
    let mut conn = get_conn().await?;
    let results: Vec<Vec<u8>> = conn.hget(key.as_ref(), fields).await?;
    let mut iter = results.into_iter();
    let mut mp = JsonMap::new();
    for i in 0..fields.len() {
        mp.insert(fields[i].into(), bytes_to_json(iter.next().unwrap()));
    }
    Ok(mp)
}

pub async fn hmget_vec<K: AsRef<str>>(key: K, fields: &[&str]) -> Result<Vec<Json>> {
    if fields.len() == 0 {
        return Ok(vec![]);
    }
    if fields.len() == 1 {
        return Ok(vec![hget(key, fields[0]).await?]);
    }
    let mut conn = get_conn().await?;
    let results: Vec<Vec<u8>> = conn.hget(key.as_ref(), fields).await?;
    Ok(results
        .into_iter()
        .map(|bytes| bytes_to_json(bytes))
        .collect())
}

// pub async fn hgetall<K: AsRef<str>> (key: K) -> Result< JsonMap > {
//     let mut conn = get_conn().await?;
//     let results : Vec<Vec<u8>> = conn.hgetall(key.as_ref()).await?;
// }

pub async fn zadd_multiple<
    K: AsRef<str>,
    M: ToRedisArgs + Send + Sync,
    MM: ToRedisArgs + Send + Sync,
>(
    key: K,
    items: &[(M, MM)],
) -> Result<()> {
    let mut conn = get_conn().await?;
    Ok(conn.zadd_multiple(key.as_ref(), items).await?)
}

pub async fn zrange<K: AsRef<str>>(key: K, l: isize, r: isize) -> Result<Vec<Json>> {
    let mut conn = get_conn().await?;
    let jsons = bytes_vec_to_json_vec(conn.zrange(key.as_ref(), l, r).await?);
    Ok(jsons)
}

pub async fn zrevrange<K: AsRef<str>>(key: K, l: isize, r: isize) -> Result<Vec<Json>> {
    let mut conn = get_conn().await?;
    let jsons = bytes_vec_to_json_vec(conn.zrevrange(key.as_ref(), l, r).await?);
    Ok(jsons)
}

pub async fn zrange_withscores<K: AsRef<str>>(
    key: K,
    l: isize,
    r: isize,
) -> Result<BTreeMap<String, i64>> {
    let mut conn = get_conn().await?;
    let results: BTreeMap<String, i64> = conn.zrange_withscores(key.as_ref(), l, r).await?;
    Ok(results)
}

pub async fn zrangebyscore<
    K: AsRef<str>,
    M: ToRedisArgs + Send + Sync,
    MM: ToRedisArgs + Send + Sync,
>(
    key: K,
    low: M,
    high: MM,
) -> Result<Vec<String>> {
    let mut conn = get_conn().await?;
    let res: Vec<String> = conn.zrangebyscore(key.as_ref(), low, high).await?;
    Ok(res)
}

pub async fn zrembyscore<K: AsRef<str>, M: ToRedisArgs + Send + Sync>(
    key: K,
    min: M,
    max: M,
) -> Result<()> {
    let mut conn = get_conn().await?;
    conn.zrembyscore(key.as_ref(), min, max).await?;
    Ok(())
}

pub async fn zcount<K: AsRef<str>, M: ToRedisArgs + Send + Sync, MM: ToRedisArgs + Send + Sync>(
    key: K,
    min: M,
    max: MM,
) -> Result<u32> {
    let mut conn = get_conn().await?;
    Ok(conn.zcount(key.as_ref(), min, max).await?)
}

pub async fn zcard<K: AsRef<str>>(key: K) -> Result<i64> {
    let mut conn = get_conn().await?;
    Ok(conn.zcard(key.as_ref()).await?)
}

pub async fn rpush<K: AsRef<str>>(key: K, v: &Json) -> Result<()> {
    let mut conn = get_conn().await?;
    Ok(conn.rpush(key.as_ref(), v.to_string()).await?)
}

pub async fn lrange<K: AsRef<str>>(key: K, l: isize, r: isize) -> Result<Vec<Json>> {
    let mut conn = get_conn().await?;
    let v = bytes_vec_to_json_vec(conn.lrange(key.as_ref(), l, r).await?);
    Ok(v)
}

pub async fn lrange_map<K: AsRef<str>>(key: K, l: isize, r: isize) -> Result<Vec<JsonMap>> {
    let mut conn = get_conn().await?;
    let v = bytes_vec_to_json_map_vec(conn.lrange(key.as_ref(), l, r).await?);
    Ok(v)
}

pub async fn cmd<S: AsRef<str>>(c: Vec<S>) -> Result<Vec<u8>> {
    let mut conn = get_conn().await?;
    let mut x = redis::cmd(c[0].as_ref());
    for i in 1..c.len() {
        x.arg(c[i].as_ref());
    }
    Ok(x.query_async(&mut conn as &mut Connection).await?)
}

pub async fn multi_cmd<S: AsRef<str>>(c: Vec<Vec<S>>) -> Result<()> {
    let mut pipe = redis::pipe();
    let mut conn = get_conn().await?;
    c.into_iter().for_each(|x| {
        pipe.cmd(x[0].as_ref());
        for i in 1..x.len() {
            pipe.arg(x[i].as_ref());
        }
        pipe.ignore();
    });
    pipe.query_async(&mut conn as &mut Connection).await?;
    Ok(())
}
