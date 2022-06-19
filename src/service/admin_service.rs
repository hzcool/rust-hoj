use super::forms::from_validation_errors;
use super::user_service::SubmitCodeBody;
use crate::constants;
use crate::dao::{contest_dao as cd, crud, problem_dao as pd, tsubmission_dao as tsd};
use crate::model::contest::Contest;
use crate::model::problem::{Problem, TestCase};
use crate::types::{
    error_response::ErrorResponse,
    links::{JsonMap, ResponseResult},
    response::Response
};
use std::str::FromStr;
use serde_json::Value as Json;
use validator::Validate;
use axum::{
    response::{
        Response as AxumResponse,
        IntoResponse
    },
    body::{Full},
    http::{header::{HeaderMap, CONTENT_TYPE}, StatusCode},
    extract::{ Json as AxumJson, Path, Multipart, ContentLengthLimit, Extension}
};

use futures::{AsyncWriteExt};
use crate::constants::ZIPPER_PATH;
use crate::utils::file;
use crate::utils::jwt::UserToken;


pub async fn for_admin_page() -> AxumResponse {
    Response::from_msg("ok").into_response()
}

pub async fn create_problem(AxumJson(mp): AxumJson<JsonMap>) -> ResponseResult {
    let mut problem: Problem = mp.into();
    problem.created_at = chrono::Local::now().timestamp_millis();

    if let Err(e) = problem.validate() {
        return Err(ErrorResponse::bad_request_with_str(
            from_validation_errors(e).as_str(),
        ));
    }
    Ok(Response::from(pd::insert(problem).await?).into_response())
}

pub async fn clone_problem(Path(pid): Path<i64>) -> ResponseResult {

    // 拷贝题目
    let mut problem = crud::get_object::<Problem>(pid).await?;
    let src_data_dir = std::path::PathBuf::from_str(
        format!("{}/{}", *constants::TEST_CASE_DIR, problem.index).as_str(),
    )
        .unwrap();
    problem.id = 0;
    problem.is_open = !problem.is_open;
    let last_index = pd::last_index(problem.is_open).await?;
    problem.index = format!(
        "{}{}",
        &last_index[0..1],
        pd::score_of_index(last_index.as_str()) + 1
    );
    let tar_data_dir = std::path::PathBuf::from_str(
        format!("{}/{}", *constants::TEST_CASE_DIR, problem.index).as_str(),
    )
        .unwrap();
    problem.created_at = chrono::Local::now().timestamp_millis();
    problem.all_count = 0;
    problem.accepted_count = 0;

    if src_data_dir.exists() {
        async_process::Command::new("/usr/bin/cp")
            .arg("-r")
            .arg(src_data_dir.as_os_str())
            .arg(tar_data_dir.as_os_str())
            .output()
            .await
            .unwrap();
    }

    Ok(Response::from(pd::insert(problem).await?).into_response())
}

pub async fn delete_problem(Path(pid): Path<i64>) -> ResponseResult {
    let index = crud::get_one_value::<Problem, String>(pid, "index").await?;

    //数据位置
    let src_data_dir =
        std::path::PathBuf::from_str(format!("{}/{}", *constants::TEST_CASE_DIR, index).as_str())
            .unwrap();
    pd::delete(pid).await?;
    async_process::Command::new("rm")
        .arg("-rf")
        .arg(src_data_dir.as_os_str())
        .output()
        .await
        .unwrap();
    Ok(Response::from("ok").into_response())
}

pub async fn upload_test_cases(
    Path(index):  Path<String>,
    ContentLengthLimit(mut multipart): ContentLengthLimit<Multipart, {64 * 1024 * 1024}>
) -> ResponseResult {
    let temp_dir = tempfile::tempdir_in(constants::TEMP_DIR.as_str()).unwrap();
    let zip_path = temp_dir.path().join(format!("{}.zip", index.as_str()));
    let mut end = false;
    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap_or("");
        if name == "end" {
            end = true;
            break;
        }
        if name != "zip" {
            continue;
        }
        let mut zip = async_fs::File::create(zip_path.as_path()).await.unwrap();
        let data = field.bytes().await.unwrap();
        zip.write_all(data.as_ref()).await.unwrap();
        zip.flush().await.unwrap();
    }
    if end {
        let data = pd::handle_zip_data(index.as_str(), zip_path.as_path()).await?;
        temp_dir.close().unwrap();
        return Ok(Response::from(data).into_response());
    }
    temp_dir.close().unwrap();
    Err(ErrorResponse::bad_request_with_str("请求未处理，可能遭到了中断"))
}

pub async fn show_one_test_case(Path((index, input_name, output_name)): Path<(String, String, String)>) -> ResponseResult {
    let dir = std::path::Path::new(constants::TEST_CASE_DIR.as_str()).join(index);
    let mut mp = JsonMap::new();
    let input = file::async_get_content(dir.join(input_name).as_path()).await?;
    let output = file::async_get_content(dir.join(output_name).as_path()).await?;
    mp.insert("input".to_string(), Json::String(input));
    mp.insert("output".to_string(), Json::String(output));
    Ok(Response::from(mp).into_response())
}

pub async fn delete_test_cases(
    Path(index): Path<String>,
    AxumJson(test_cases): AxumJson<Vec<TestCase>>,
) -> ResponseResult {
    let dir = std::path::Path::new(constants::TEST_CASE_DIR.as_str()).join(index.as_str());
    if test_cases.len() == 1 {
        let item = test_cases.iter().next().unwrap();
        async_fs::remove_file(dir.join(item.input_name.as_str()))
            .await
            .unwrap_or(());
        async_fs::remove_file(dir.join(item.output_name.as_str()))
            .await
            .unwrap_or(());
    } else {
        async_fs::remove_dir_all(dir.as_path()).await.unwrap_or(());
    }
    let data = pd::make_test_case_info(index.as_str(), dir.as_path()).await?;
    Ok(Response::from(data).into_response())
}

pub async fn download_test_cases(
    Path(index): Path<String>,
) -> ResponseResult {
    let temp_dir = tempfile::tempdir_in(constants::TEMP_DIR.as_str()).unwrap();
    let tar_path = temp_dir.path().join(format!("{}.zip", index));
    let src_dir = std::path::Path::new(constants::TEST_CASE_DIR.as_str()).join(index);
    async_process::Command::new(ZIPPER_PATH.as_os_str())
        .arg("zip")
        .arg(src_dir.as_os_str())
        .arg(tar_path.as_os_str())
        .output()
        .await
        .map_err(|e| {
            ErrorResponse::server_error_with_str(format!("压缩文件出错: {}", e).as_str())
        })?;
    let data = crate::utils::file::async_get_buffer(&tar_path).await?;
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "application/octet-stream".parse().unwrap());
    let response = AxumResponse::new(Full::from(data));
    let (mut parts, body) = response.into_parts();
    parts.status = StatusCode::OK;
    parts.headers = headers;
    Ok(AxumResponse::from_parts(parts, body).into_response())
}

pub async fn create_contest(AxumJson(json_map): AxumJson<JsonMap>) -> ResponseResult {
    let contest: Contest = json_map.into();
    if contest.length <= 0 {
        return Err(ErrorResponse::bad_request_with_str("比赛长度不能小于0!!!"));
    }
    if !contest.is_open && contest.password == "" {
        return Err(ErrorResponse::bad_request_with_str("未设置比赛密码"));
    }
    Ok(Response::from(cd::insert(contest).await?).into_response())
}

pub async fn submit_ts_code(token_data: Extension<UserToken>, AxumJson(bd): AxumJson<SubmitCodeBody>) -> ResponseResult {
    let sid = tsd::handle_tsubmission(token_data.id, bd.pid, bd.lang, bd.code).await?;
    Ok(Response::from(sid).into_response())
}

pub async fn get_ts_status(Path(sid): Path<i64>) -> ResponseResult {
    Ok(Response::from(tsd::get_status(sid).await?).into_response())
}