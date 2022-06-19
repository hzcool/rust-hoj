use crate::config::env;
use crate::constants;
use chrono::Local;
use jsonwebtoken::{errors::ErrorKind, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use lazy_static::lazy_static;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

lazy_static! {
    static ref HEADER: Header = Header::new(Algorithm::HS256);
    static ref VALIDATION: Validation = Validation::new(Algorithm::HS256);
    static ref SECRET: String = env::get_key("JWT_HS256_KEY");
    static ref DK: DecodingKey<'static> = DecodingKey::from_secret(SECRET.as_bytes());
    static ref EK: EncodingKey = EncodingKey::from_secret(SECRET.as_bytes());
    static ref PERIOD_OF_VALIDITY: i64 = env::get_key("PERIOD_OF_VALIDITY").parse::<i64>().unwrap();
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserToken {
    pub iat: i64, // Issued at (as UTC timestamp)  jwt的签发时间
    pub exp: i64, // Required (validate_exp defaults to true in validation). Expiration time (as UTC timestamp)
    pub id: i64,
    pub user: String,   //用户名
    pub role: String,   //身份
    pub privilege: u64, //权力
}

impl UserToken {
    pub fn from(id: i64, user: String, role: String) -> Self {
        let now = Local::now().timestamp();
        UserToken {
            iat: now,
            exp: now + *PERIOD_OF_VALIDITY,
            id,
            user,
            role,
            privilege: 0,
        }
    }

    pub fn is_admin(&self) -> bool {
        self.role == constants::ADMIN || self.role == constants::SUPER_ADMIN
    }

    pub fn is_super_admin(&self) -> bool {
        self.role == constants::SUPER_ADMIN
    }
}

pub fn encode<T: Serialize>(claims: &T) -> String {
    jsonwebtoken::encode(&*HEADER, claims, &EK).unwrap()
}

pub fn decode<T: DeserializeOwned>(token: &str) -> Result<T, ErrorKind> {
    let t = jsonwebtoken::decode::<T>(token, &DK, &*VALIDATION);
    match t {
        Ok(res) => Ok(res.claims),
        Err(e) => Err(e.into_kind()),
    }
}
