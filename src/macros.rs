#[macro_export]
macro_rules! json_map {
    ($($key : expr => $value: expr ),*) => {{
        let mut mp = serde_json::Map::new();
        $(mp.insert($key.into(), serde_json::json!($value));)*
        mp
    }};

    ($($key:pat => $value:pat), *) => {{
        let mut mp = serde_json::Map::new();
        $(mp.insert($key.into(), serde_json::json!($value));)*
        mp
    }};
}
