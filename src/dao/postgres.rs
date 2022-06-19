use deadpool::managed::Object;
use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod};
use std::str::FromStr;
use anyhow::Result;
use futures::StreamExt;

use crate::config::env;
use crate::constants;
use lazy_static::lazy_static;
lazy_static! {
    static ref POOL: Pool = config_pg_pool();
}

pub fn config_pg_pool() -> Pool {
    let cfg = tokio_postgres::Config::from_str(env::database_url().as_str()).unwrap();
    let mgr_config = ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    };
    let mgr = Manager::from_config(cfg, tokio_postgres::NoTls, mgr_config);
    Pool::new(mgr, constants::POSTGRES_POOL_SIZE)
}

pub async fn get_pg_connect() -> Result<Object<Manager>> {
    Ok(POOL.get().await?)
}

pub async fn init_tables() -> Result<()> {
    let sql_dir = crate::config::env::get_key("SQL_DIR");
    let conn = get_pg_connect().await?;
    let mut entries = async_walkdir::WalkDir::new(std::path::Path::new(sql_dir.as_str()));
    loop {
        match entries.next().await {
            Some(Ok(entry)) => {
                let meta_data = entry.metadata().await.unwrap();
                if meta_data.is_dir() || entry.path().extension().unwrap() != "sql" {
                    continue;
                }
                let sql = crate::utils::file::async_get_content(entry.path().as_path()).await?;
                conn.batch_execute(sql.as_str()).await.unwrap_or_else(|e|{
                    println!("{:?} {}", entry.path().file_name().unwrap(), e);
                })
            }
            _ => break,
        }
    }
    Ok(())
}
