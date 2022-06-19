pub mod config;
pub mod dao;
pub mod model;
pub mod constants;
pub mod types;
pub mod macros;
pub mod utils;
pub mod service;
pub mod middleware;


#[tokio::main]
async fn main() {
    config::init::init().await.expect("初始化失败");

    let app = config::routes::config_routes();
    axum::Server::bind(&config::env::addr().as_str().parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
