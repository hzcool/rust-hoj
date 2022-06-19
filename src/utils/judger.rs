use crate::constants;
use crate::model::problem::{Checker, SpjConfig, TestCase};
use crate::model::submission::CaseResult;
use anyhow::Result;
use hyper::{client::Client, client::HttpConnector, Body, Method, Request, Uri};
use lazy_static::lazy_static;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::{mpsc, mpsc::Receiver, mpsc::Sender, Mutex};

lazy_static! {
    pub static ref CLIENT: Client<HttpConnector, Body> = Client::new();
    pub static ref JUDGE_URI: Uri =
        format!("{}{}", crate::config::env::get_key("JUDGE_URL"), "/judge")
            .parse::<Uri>()
            .unwrap();
    pub static ref PING_URI: Uri =
        format!("{}{}", crate::config::env::get_key("JUDGE_URL"), "/ping")
            .parse::<Uri>()
            .unwrap();
    pub static ref ACCESS_TOKEN: String = crate::config::env::get_key("JUDGE_ACCESS_TOKEN");
    static ref CH: Mutex<(Sender<usize>, Receiver<usize>)> =
        Mutex::new(mpsc::channel::<usize>(constants::MAX_JUDGE_TASKS));
    static ref SENDER: Mutex<Sender<usize>> = Mutex::new(futures::executor::block_on(async {
        CH.lock().await.0.clone()
    }));
}

pub async fn acquire_judge_chance() {
    SENDER.lock().await.send(1).await.unwrap()
}
pub async fn release_judge_chance() {
    let _ = CH.lock().await.1.recv().await.unwrap();
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct JudgeConfig {
    pub lang: String, //语言

    pub src: String, //源码

    pub max_cpu_time: i32, //最大cpu时间

    pub max_memory: i32, //最大内存

    pub io_dir: String, // 测试用例文件夹

    pub test_cases: Vec<TestCase>, //测试用例

    pub checker: Option<Checker>, // 检查器

    pub spj_config: Option<SpjConfig>, // special judger

    pub seccomp_rule: Option<String>, //  权限规则

    pub resource_rule: Option<i8>, // 资源限制规则

    pub test_all: Option<bool>,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct JudgeResult {
    pub compile_info: String,     //编译信息
    pub case_count: i16,          //用例总数
    pub pass_count: i16,          //通过的数量
    pub time: i32,                //用时
    pub memory: i32,              //内存消耗
    pub total_time: i32,          //所有样例总用时
    pub status: i32,              //结果
    pub error: String,            //错误信息
    pub results: Vec<CaseResult>, //各个用例的结果
}
pub async fn judge(config: JudgeConfig) -> Result<JudgeResult> {
    let js = json!(config);
    let req = Request::builder()
        .uri(&*JUDGE_URI)
        .method(Method::POST)
        .header("content-type", "application/json")
        .header("ACCESS_TOKEN", ACCESS_TOKEN.as_str())
        .body(Body::from(js.to_string()))
        .expect("构建 judge 请求出错");
    let res = CLIENT.request(req).await.expect("judge 请求失败");
    if !res.status().is_success() {
        return Err(anyhow::Error::msg("judge 请求失败"));
    }
    let (_, body) = res.into_parts();
    let buf = hyper::body::to_bytes(body).await?;
    Ok(serde_json::from_slice(buf.to_vec().as_slice())
        .map_err(|e| anyhow::Error::msg(format!("测评结果解析失败 : {}", e)))?)
}

pub async fn ping() {
    let req = Request::builder()
        .uri(&*PING_URI)
        .method(Method::POST)
        .header("content-type", "application/json")
        .header("ACCESS_TOKEN", ACCESS_TOKEN.as_str())
        .body(Body::from(""))
        .expect("请求失败");
    let res = CLIENT.request(req).await.expect("gg");
    let (_, body) = res.into_parts();
    let buf = hyper::body::to_bytes(body).await.unwrap();
    println!("{}", String::from_utf8(buf.to_vec()).unwrap());
}

pub async fn test() {
    for i in 0..100 {
        tokio::spawn(async move {
            acquire_judge_chance().await;
            let t = {
                let mut rng = rand::thread_rng();
                rng.gen_range(1u64..5u64)
            };
            tokio::time::sleep(tokio::time::Duration::from_secs(t)).await;
            println!("{} : {}", i, t);
            release_judge_chance().await;
        });
    }
}
