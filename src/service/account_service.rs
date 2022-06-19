use crate::service::forms::{from_validation_errors, LoginForm, RegisterForm};
use crate::constants;
use crate::dao::crud;
use crate::model::user::User;
use crate::types::{error_response::ErrorResponse, links::ResponseResult, response::Response, links::JsonMap};
use crate::utils::{generator, jwt};
use validator::Validate;
use serde::Deserialize;
use axum::{
    extract::{Form, Path, Extension, Json},
    http::{HeaderValue},
    response::IntoResponse
};

pub async fn register(rf: Form<RegisterForm>) -> ResponseResult {
    if let Err(e) = rf.0.validate() {
        return Err(ErrorResponse::forbidden_with_str(
            from_validation_errors(e).as_str(),
        ));
    }
    if crud::count::<User>(crate::json_map!("username" => rf.username)).await? > 0 {
        return Err(ErrorResponse::forbidden_with_str("用户名已存在"));
    }
    if crud::count::<User>(crate::json_map!("email" => rf.email)).await? > 0 {
        return Err(ErrorResponse::forbidden_with_str("邮箱已被注册"));
    }
    let mut user = User::default();
    user.username = rf.username.clone();
    user.email = rf.email.clone();
    user.password = rf.password.clone();
    user.school = rf.school.clone();
    user.avatar = generator::random_avatar();
    user.created_at = Some(chrono::Local::now().timestamp_millis());
    let id = crud::insert_without_dev(&user).await?;
    account_info_response(id).await
}

pub async fn login(lf: Form<LoginForm>) -> ResponseResult {
    if let Err(e) = lf.0.validate() {
        return Err(ErrorResponse::forbidden_with_str(
            from_validation_errors(e).as_str(),
        ));
    }

    let pwd = lf.password.clone();
    let func = |id: i64| async move {
        if pwd
            != crud::get_one_value::<User, String>(id, "password")
            .await
            .unwrap()
        {
            return Err(ErrorResponse::forbidden_with_str("密码错误"));
        }
        account_info_response(id).await
    };

    if let Ok(id) = crud::get_id::<User>(crate::json_map!("username" => lf.username_or_email)).await
    {
        return func(id).await;
    }
    if let Ok(id) = crud::get_id::<User>(crate::json_map!("email" => lf.username_or_email)).await {
        return func(id).await;
    }
    Err(ErrorResponse::forbidden_with_str("用户名或邮箱不存在"))
}

pub async fn logout() -> ResponseResult {
    Ok(Response::from_msg("退出成功").into_response())
}

pub async fn autologin(token_data: Extension<jwt::UserToken>) -> ResponseResult {
    account_info_response(token_data.id).await
}

pub async fn account_info_response(id: i64) -> ResponseResult {
    let info = crud::get_columns::<User>(id, &["id", "username", "avatar", "role"]).await?;
    let token_data = jwt::UserToken::from(
        id,
        info.get("username").unwrap().as_str().unwrap().to_string(),
        info.get("role").unwrap().as_str().unwrap().to_string(),
    );
    let token = jwt::encode(&token_data);

    let mut resp = Response::from(info).into_response();
    resp.headers_mut().insert(
        constants::HEAD_TOKEN_NAME,
        HeaderValue::from_str(token.as_str()).unwrap()
    );
    Ok(resp)
}

#[derive(Deserialize)]
pub struct UserupdateBody {
    pub avatar: Option<String>,
    pub description: Option<String>,
    pub old_password: Option<String>,
    pub new_password: Option<String>,
    pub school: Option<String>,
    pub email: Option<String>,
}
pub async fn update_user_info(
    token_data: Extension<jwt::UserToken>,
    Path(uid): Path<i64>,
    Json(body): Json<UserupdateBody>,
) -> ResponseResult {
    if token_data.id != 1 && token_data.id != uid {
        return Err(ErrorResponse::forbidden_with_str("无权修改"));
    }
    let mut update_map = JsonMap::new();
    if let Some(email) = body.email {
        if crud::count::<User>(crate::json_map!("email" => email)).await? > 0 {
            return Err(ErrorResponse::forbidden_with_str("邮箱已被注册"));
        }
        let email_check = super::forms::EmailCheck {
            email: email.clone(),
        };
        if let Err(e) = email_check.validate() {
            return Err(ErrorResponse::forbidden_with_str(
                from_validation_errors(e).as_str(),
            ));
        }
        update_map.insert("email".into(), serde_json::Value::String(email));
    }
    if let Some(avatar) = body.avatar {
        update_map.insert("avatar".into(), serde_json::Value::String(avatar));
    }
    if let Some(description) = body.description {
        update_map.insert("description".into(), serde_json::Value::String(description));
    }
    if let Some(school) = body.school {
        update_map.insert("school".into(), serde_json::Value::String(school));
    }
    if let Some(new_password) = body.new_password {
        let old_password = body.old_password.unwrap_or("".into());
        let pwd = crud::get_one_value::<User, String>(uid, "password").await?;
        if old_password != pwd {
            return Err(ErrorResponse::forbidden_with_str("密码错误"));
        }
        let pwd_check = super::forms::PasswordCheck {
            password: new_password.clone(),
        };
        if let Err(e) = pwd_check.validate() {
            return Err(ErrorResponse::forbidden_with_str(
                from_validation_errors(e).as_str(),
            ));
        }
        update_map.insert("password".into(), serde_json::Value::String(new_password));
    }

    crud::update_by_map_without_dev::<User>(uid, &update_map).await?;
    Ok(Response::from_msg("ok").into_response())
}