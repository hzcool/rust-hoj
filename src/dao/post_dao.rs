use crate::model::post::*;
use crate::model::comment::*;
use crate::types::links::JsonMap;
use crate::dao::{crud, redis_db, postgres};
use crate::model::traits::*;
use anyhow::Result;
use serde_json::{json, Value as Json};
use crate::json_map;
use crate::model::user::User;


pub async fn add_extra_user_info_to_map(mp: &mut JsonMap) -> Result<()> {
    let uid = mp.get("uid").unwrap().as_i64().unwrap();
    let values = crud::get_values::<User>(uid, &["username", "avatar"]).await?;
    let mut it = values.into_iter();
    mp.insert("author".to_string(), it.next().unwrap());
    mp.insert("avatar".to_string(), it.next().unwrap());
    Ok(())
}

pub async fn add_info_to_comment_map(mp: &mut JsonMap) -> Result<()> {
    add_extra_user_info_to_map(mp).await?;
    let reply_id = mp.get("reply_id").unwrap().as_i64().unwrap();
    if reply_id != 0 {
        let values = crud::get_values::<User>(reply_id, &["username", "avatar"]).await?;
        let mut it = values.into_iter();
        mp.insert("reply_author".to_string(), it.next().unwrap());
        mp.insert("reply_avatar".to_string(), it.next().unwrap());
    }
    Ok(())
}

pub async fn cache_post_zset(cid: i64, pid: i64) -> Result<String> {
    let zkey = Post::zset_key_with_two_deps(&cid, &pid);
    if !redis_db::exists(zkey.as_str()).await? {
        let sql = Post::sql_with_two_deps(&cid, &pid);
        let conn = postgres::get_pg_connect().await?;
        let rows = conn.query(sql.as_str(), &[]).await?;
        if rows.is_empty() {
            return Ok(zkey)
        }
        let mut items = Vec::new();
        for row in rows {
            let mut mp: JsonMap = crud::pg_row_to_json_map(row);
            add_extra_user_info_to_map(&mut mp).await.unwrap();
            items.push( (Post::score_from_map(&mp), Json::Object(mp).to_string()));
        }
        redis_db::zadd_multiple(zkey.as_str(), items.as_slice()).await?;
        redis_db::expire(zkey.as_str(), Post::zset_expire()).await?;
    }
    Ok(zkey)
}

pub async fn cache_one_post(post_id: i64) -> Result<String> {
    let post = crud::get_object::<Post>(post_id).await?;
    let mut value = json!(post);
    let mp = value.as_object_mut().unwrap();
    add_extra_user_info_to_map(mp).await?;
    let zkey = Post::zset_key_with_two_deps(&post.cid, &post.pid);
    redis_db::zadd_multiple(zkey.as_str(), &[(post_id, value.to_string())]).await?;
    redis_db::expire(zkey.as_str(), Post::zset_expire()).await?;
    Ok(zkey)
}

pub async fn find(cid: i64, pid: i64, desc: Option<bool>, limit: Option<i32>, offset: Option<i32>) -> Result<Vec<JsonMap>> {
    let l = offset.unwrap_or(0) as isize;
    let r = limit.unwrap_or(0) as isize + l - 1;
    let zkey = cache_post_zset(cid, pid).await?;
    let res = match desc.unwrap_or(false) {
        true => redis_db::zrange(zkey.as_str(), l, r).await?,
        _ => redis_db::zrevrange(zkey.as_str(), l, r).await?
    };
    Ok(res.into_iter()
        .map(|item| serde_json::from_value(item).unwrap())
        .collect())
}

pub async fn insert(post: Post) -> Result<i64> {
    let value = json!(post);
    let mp = value.as_object().unwrap();
    let id = crud::insert_by_map::<Post>(mp).await?;
    cache_one_post(id).await?;
    Ok(id)
}

pub async fn delete(post_id: i64) -> Result<()> {
    let values = crud::get_values::<Post>(post_id, &["cid", "pid"]).await?;
    let (cid, pid) = (values[0].as_i64().unwrap(), values[1].as_i64().unwrap());
    let zkey = Post::zset_key_with_two_deps(&cid, &pid);
    redis_db::zrembyscore(zkey.as_str(), post_id, post_id).await.unwrap_or(());
    let key = Post::redis_key(post_id);
    redis_db::del(key.as_str()).await.unwrap_or(());
    let conn = postgres::get_pg_connect().await?;
    let sql = format!("DELETE FROM \"{}\" WHERE id = {}", Post::table_name(), post_id);
    let _ = conn.query(sql.as_str(), &[]).await?;
    Ok(())
}

pub async fn update(post_id: i64, update_map: JsonMap) -> Result<()> {
    crud::update_by_map_for_sql::<Post>(post_id, &update_map).await?;
    let values = crud::get_values::<Post>(post_id, &["cid", "pid"]).await?;
    let (cid, pid) = (values[0].as_i64().unwrap(), values[1].as_i64().unwrap());
    let zkey = Post::zset_key_with_two_deps(&cid, &pid);
    redis_db::zrembyscore(zkey.as_str(), post_id, post_id).await.unwrap_or(());
    cache_one_post(post_id).await?;
    Ok(())
}


pub async fn cache_one_comment(comment_id: i64) -> Result<String> {
    let comment = crud::get_object::<Comment>(comment_id).await?;
    let mut value = json!(comment);
    let mp = value.as_object_mut().unwrap();
    add_info_to_comment_map(mp).await?;
    let zkey = Comment::zset_key_with_one_dep(&comment.post_id);
    redis_db::zadd_multiple(zkey.as_str(), &[(comment_id, value.to_string())]).await?;
    redis_db::expire(zkey.as_str(), Comment::zset_expire()).await?;
    Ok(zkey)
}

pub async fn cache_comment_zset(post_id: i64) -> Result<String> {
    let zkey = Comment::zset_key_with_one_dep(&post_id);
    if !redis_db::exists(zkey.as_str()).await? {
        let sql = Comment::sql_with_one_dep(&post_id);
        let conn = postgres::get_pg_connect().await?;
        let rows = conn.query(sql.as_str(), &[]).await?;
        if rows.is_empty() {
            return Ok(zkey)
        }
        let mut items = Vec::new();
        for row in rows {
            let mut mp: JsonMap = crud::pg_row_to_json_map(row);
            add_info_to_comment_map(&mut mp).await.unwrap();
            items.push( (Comment::score_from_map(&mp), Json::Object(mp).to_string()));
        }
        redis_db::zadd_multiple(zkey.as_str(), items.as_slice()).await?;
        redis_db::expire(zkey.as_str(), Comment::zset_expire()).await?;
    }
    Ok(zkey)
}

pub async fn find_comments(post_id: i64, limit: Option<i32>, offset: Option<i32>) -> Result<Vec<JsonMap>> {
    let l = offset.unwrap_or(0) as isize;
    let r = limit.unwrap_or(0) as isize + l - 1;
    let zkey = cache_comment_zset(post_id).await?;
    let res = redis_db::zrange(zkey.as_str(), l, r).await?;
    Ok(res.into_iter()
        .map(|item| serde_json::from_value(item).unwrap())
        .collect())
}

pub async fn add_comment(comment: Comment) -> Result<()> {
    let value = json!(comment);
    let mp = value.as_object().unwrap();
    let id = crud::insert_by_map::<Comment>(mp).await?;
    cache_one_comment(id).await?;
    let comment_count = crud::get_one_value::<Post, u32>(comment.post_id, "comment_count").await? + 1;
    update(comment.post_id, json_map!("comment_count" => comment_count)).await?;
    Ok(())
}






