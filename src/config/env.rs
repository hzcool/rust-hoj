pub fn addr() -> String {
    std::env::var("addr").expect("Not Find addr to bind")
}

pub fn database_url() -> String {
    std::env::var("DATABASE_URL").expect("No DATABASE_URL in env")
}

pub fn redis_url() -> String {
    std::env::var("REDIS_URL").expect("No REDIS_URL in env")
}

pub fn get_key(key: &str) -> String {
    std::env::var(key).expect(format!("NO SUCH KEY {} in env", key).as_str())
}
