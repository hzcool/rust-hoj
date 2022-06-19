
use crate::config::env;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref TEMP_DIR: String = env::get_key("TEMP_DIR");
    pub static ref TEST_CASE_DIR: String = env::get_key("TEST_CASE_DIR");
    pub static ref ZIPPER_PATH: std::ffi::OsString = std::ffi::OsString::from(env::get_key("ZIPPER_PATH"));
}

// Headers
pub const AUTHORIZATION: &str = "Authorization";
pub const HEAD_TOKEN_NAME: &str = "token";

//table_name
pub const USER_TABLE_NAME: &str = "user";
pub const PROBLEM_TABLE_NAME: &str = "problem";
pub const SUBMISSION_TABLE_NAME: &str = "submission";
pub const CONTEST_TABLE_NAME: &str = "contest";
pub const TEAM_TABLE_NAME: &str = "team";
pub const CSUBMISSION_TABLE_NAME: &str = "csubmission";
pub const TSUBMISSION_TABLE_NAME: &str = "tsubmission";
pub const POST_TABLE_NAME: &str = "post";
pub const STAR_TABLE_NAME: &str = "star";
pub const COMMENT_TABLE_NAME: &str = "comment";

pub const MAX_JUDGE_TASKS: usize = 2;

pub const POSTGRES_POOL_SIZE: usize = 8;
pub const REDIS_POOL_SIZE: u64 = 10;

//role
pub const SUPER_ADMIN: &str = "super_admin";
pub const ADMIN: &str = "admin";
pub const USER: &str = "user";
pub const VISITOR: &str = "visitor";

// 比赛结束后等待10分钟后关闭测评队列
pub const WAITING_TIME_CLOSE_JUDGING_QUEUE: u64 = 600;

pub const AVATARS: [&str; 60] = [
    "https://www.helloimg.com/images/2020/07/27/1041d84b6996f3e71.jpg",
    "https://www.helloimg.com/images/2020/07/27/22bc7f41d2fdb9e5b.jpg",
    "https://www.helloimg.com/images/2020/07/27/36f2c2db317852c3b.jpg",
    "https://www.helloimg.com/images/2020/07/27/4df9611d5bfe21c7e.jpg",
    "https://www.helloimg.com/images/2020/07/27/5b67618b3dd028f80.jpg",
    "https://www.helloimg.com/images/2020/07/27/6a0de64d5553e41c8.jpg",
    "https://www.helloimg.com/images/2020/07/27/75e82dfc4da6e912e.jpg",
    "https://www.helloimg.com/images/2020/07/27/8774bfc43339c3c53.jpg",
    "https://www.helloimg.com/images/2020/07/27/978908a0f67d32310.jpg",
    "https://www.helloimg.com/images/2020/07/27/10af9e2f1e59c9b6c8.jpg",
    "https://www.helloimg.com/images/2020/07/27/111f9af15ee2a47b61.jpg",
    "https://www.helloimg.com/images/2020/07/27/12d521a26bc7c48dbc.jpg",
    "https://www.helloimg.com/images/2020/07/27/13c5ca7e2f6f779978.jpg",
    "https://www.helloimg.com/images/2020/07/27/147a1b9f6658e8e513.jpg",
    "https://www.helloimg.com/images/2020/07/27/15c08edd1a2bd11d41.jpg",
    "https://www.helloimg.com/images/2020/07/27/16e25afc8bbbee93ec.jpg",
    "https://www.helloimg.com/images/2020/07/27/17dda9ee71907fa223.jpg",
    "https://www.helloimg.com/images/2020/07/27/1889121832ba812a47.jpg",
    "https://www.helloimg.com/images/2020/07/27/19f4e835b6f2b0ef54.jpg",
    "https://www.helloimg.com/images/2020/07/27/20dfd7d95b5849ce27.jpg",
    "https://www.helloimg.com/images/2020/07/27/217f1d33ed052504ca.jpg",
    "https://www.helloimg.com/images/2020/07/27/227d8792c7cb169aa4.jpg",
    "https://www.helloimg.com/images/2020/07/27/23b143f0d64bb0a0b1.jpg",
    "https://www.helloimg.com/images/2020/07/27/24ee286f1bd696f0b9.jpg",
    "https://www.helloimg.com/images/2020/07/27/25ae5ecfd8da494f6f.jpg",
    "https://www.helloimg.com/images/2020/07/27/264f906fb598ee34b3.jpg",
    "https://www.helloimg.com/images/2020/07/27/279aa74c7e3515750c.jpg",
    "https://www.helloimg.com/images/2020/07/27/28607ecf307763dbe5.jpg",
    "https://www.helloimg.com/images/2020/07/27/29862cf80f38736fbb.jpg",
    "https://www.helloimg.com/images/2020/07/27/304e1f23c217a41999.jpg",
    "https://www.helloimg.com/images/2020/07/27/318b4daeb43fbf1802.jpg",
    "https://www.helloimg.com/images/2020/07/27/324eecc594b28814c7.jpg",
    "https://www.helloimg.com/images/2020/07/27/3374dc0ffa181269c8.jpg",
    "https://www.helloimg.com/images/2020/07/27/34b2e52c26bebca297.jpg",
    "https://www.helloimg.com/images/2020/07/27/351edc0e6875ecff00.jpg",
    "https://www.helloimg.com/images/2020/07/27/36d88376228475d6a7.jpg",
    "https://www.helloimg.com/images/2020/07/27/37b0bba53e44a2e9e3.jpg",
    "https://www.helloimg.com/images/2020/07/27/3868b2918655b0f625.jpg",
    "https://www.helloimg.com/images/2020/07/27/39ca20bd69903f96b9.jpg",
    "https://www.helloimg.com/images/2020/07/27/406309254b3bc0b79b.jpg",
    "https://www.helloimg.com/images/2020/07/27/4173e50ba17f5b1552.jpg",
    "https://www.helloimg.com/images/2020/07/27/422bed47ba3f643384.jpg",
    "https://www.helloimg.com/images/2020/07/27/432cb2351b3c0ab0f9.jpg",
    "https://www.helloimg.com/images/2020/07/27/44c14333f65acf486a.jpg",
    "https://www.helloimg.com/images/2020/07/27/4596f05f87e86e7ad3.jpg",
    "https://www.helloimg.com/images/2020/07/27/46e501f90fbfd1eb87.jpg",
    "https://www.helloimg.com/images/2020/07/27/47a520d1dca1bb5122.jpg",
    "https://www.helloimg.com/images/2020/07/27/48fadbabaa6d6c8435.jpg",
    "https://www.helloimg.com/images/2020/07/27/49bd302f8fb7928371.jpg",
    "https://www.helloimg.com/images/2020/07/27/50c4c7b87fdeaa8143.jpg",
    "https://www.helloimg.com/images/2020/07/27/519099d1fdf7e00a7a.jpg",
    "https://www.helloimg.com/images/2020/07/27/523ab9d4202496e307.jpg",
    "https://www.helloimg.com/images/2020/07/27/5395557405919005f5.jpg",
    "https://www.helloimg.com/images/2020/07/27/5495e27c8558ad2f85.jpg",
    "https://www.helloimg.com/images/2020/07/27/55eb762367578a6e2b.jpg",
    "https://www.helloimg.com/images/2020/07/27/565490a7dc3ab04ce6.jpg",
    "https://www.helloimg.com/images/2020/07/27/571f7340a288d0d640.jpg",
    "https://www.helloimg.com/images/2020/07/27/58dd9266fe40ac8616.jpg",
    "https://www.helloimg.com/images/2020/07/27/59118c1d360193294b.jpg",
    "https://www.helloimg.com/images/2020/07/27/607b22ba2a397f2511.jpg",
];