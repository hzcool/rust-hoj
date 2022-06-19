use crate::dao::{contest_dao as cd, postgres};
use std::process::exit;

pub async fn init() -> anyhow::Result<()> {
    dotenv::dotenv().expect("Failed to read .env file");
    crate::dao::redis_db::ping().await;
    postgres::get_pg_connect().await.expect("连接数据库出错");

    //postgres 建表
    if let Err(e) = postgres::init_tables().await {
        println!("{:?}", e);
        exit(-1);
    }

    //ping judger
    crate::utils::judger::ping().await;

    cd::fresh().await.expect("初始化比赛出错");
    Ok(())
}
