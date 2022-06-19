use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;
use validator::{HasLen, Validate, ValidationError, ValidationErrors};

#[derive(Validate, Serialize, Deserialize, Debug)]
pub struct RegisterForm {
    #[validate(length(min = 1, message = "用户名不能为空"))]
    #[validate(length(max = 32, message = "用户名太长"))]
    #[validate(custom = "validate_username")]
    pub username: String,

    #[validate(length(min = 6, message = "密码长度不能少于6个字符"))]
    #[validate(length(max = 16, message = "密码长度不能超过16个字符"))]
    #[validate(custom = "validate_password")]
    pub password: String,

    #[validate(email(message = "邮箱格式不正确"))]
    pub email: String,

    #[validate(length(max = 32, message = "学校名太长"))]
    pub school: String,
}

pub fn from_validation_errors(e: ValidationErrors) -> String {
    let mut err_str = "".to_string();
    let errors_map: HashMap<&'static str, &Vec<ValidationError>> = e.field_errors();
    for (_, value) in errors_map {
        for item in value {
            match item.message.as_ref() {
                None => (),
                Some(s) => {
                    if err_str.length() > 0 {
                        err_str.push_str(" , ");
                    }
                    err_str.push_str(s)
                }
            }
        }
    }
    err_str
}

#[derive(Validate, Serialize, Deserialize, Debug)]
pub struct LoginForm {
    pub username_or_email: String,
    #[validate(custom = "validate_password")]
    pub password: String,
}

fn validate_username(s: &str) -> Result<(), ValidationError> {
    if s.contains(char::is_whitespace) {
        return Err(ValidationError {
            code: Cow::from("white_space"),
            message: Some(Cow::from("用户名不能包含空字符")),
            params: Default::default(),
        });
    }
    Ok(())
}
pub fn validate_password(s: &str) -> Result<(), ValidationError> {
    if s.contains(char::is_whitespace) {
        return Err(ValidationError {
            code: Cow::from("white_space"),
            message: Some(Cow::from("密码不能包含空字符")),
            params: Default::default(),
        });
    }
    if s.contains(|c: char| !c.is_ascii()) {
        return Err(ValidationError {
            code: Cow::from("ascii"),
            message: Some(Cow::from("密码包含非ascii字符")),
            params: Default::default(),
        });
    }
    Ok(())
}

#[derive(Validate, Debug)]
pub struct EmailCheck {
    #[validate(email(message = "邮箱格式不正确"))]
    pub email: String,
}

#[derive(Validate, Debug)]
pub struct PasswordCheck {
    #[validate(length(min = 6, message = "密码长度不能少于6个字符"))]
    #[validate(length(max = 16, message = "密码长度不能超过16个字符"))]
    #[validate(custom = "validate_password")]
    pub password: String,
}

pub fn test() {
    let rf = RegisterForm {
        username: "皇子".to_string(),
        password: "21傻12".to_string(),
        email: "1234@qq.com".to_string(),
        school: "1212".to_string(),
    };
    if let Err(e) = rf.validate() {
        println!("{:?}", e);
    }
}
