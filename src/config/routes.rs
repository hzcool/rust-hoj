use crate::{
    middleware::{
        token_decode::token_decode,
        auth_login::auth_login,
        auth_admin::auth_admin,
        auth_contest_role::auth_contest_role
    },
    service::{account_service, admin_service, crud_service, user_service}
};
use axum::{
    Router,
    routing::{get, post},
};

use axum_extra::middleware::from_fn;

fn account_routes() -> Router {
    Router::new()
        .route(
            "/logout",
            get(account_service::logout).route_layer(from_fn(auth_login))
        )
        .route(
            "/autologin",
               get(account_service::autologin).route_layer(from_fn(auth_login))
        )
        .route(
            "/update_user_info/:uid",
               post(account_service::update_user_info).route_layer(from_fn(auth_login))
        )
        .route("/register", post(account_service::register))
        .route("/login", post(account_service::login))
}

pub fn admin_routes() -> Router {
    Router::new()
        .route("/for_admin_page", get(admin_service::for_admin_page))
        .route( "/create_problem", post(admin_service::create_problem))
        .route( "/clone_problem/:pid", get(admin_service::clone_problem))
        .route("/delete_problem/:pid", get(admin_service::delete_problem))
        .route( "/upload_test_cases/:index", post(admin_service::upload_test_cases))
        .route( "/delete_test_cases/:index", post(admin_service::delete_test_cases))
        .route("/show_one_test_case/:index/:input_name/:output_name", get(admin_service::show_one_test_case))
        .route("/download_test_cases/:index", get(admin_service::download_test_cases))
        .route( "/create_contest", post(admin_service::create_contest))
        .route("/submit_tsubmission", post(admin_service::submit_ts_code))
        .route("/ts_status/:sid", get(admin_service::get_ts_status))
}

pub fn crud_routes() -> Router {
    Router::new()
        .nest(
            "/problem",
            Router::new()
                .route("/last_index/:is_open", get(crud_service::last_index))
                .route("/id", get(crud_service::get_problem_id))
        )
        .route("/find/:table_name", post(crud_service::find))
        .route("/count/:table_name", get(crud_service::count))
        .route("/get/:table_name", post(crud_service::get))
        .route("/update/:table_name/:id", post(crud_service::update).route_layer(from_fn(auth_admin)))
}

pub fn user_routes() -> Router {
    Router::new()
        .route("/get_user_info/:uid", get(user_service::get_user_info))
        .route("/get_user_all_submissions_status_info/:uid", get(user_service::get_user_all_submissions_status_info))
        .nest(
            "/sm",
            Router::new()
                .route("/submit_code", post(user_service::submit_code))
                .route("/last/:pid", get(user_service::get_last_submit))
                .route("/status/:id", get(user_service::get_status))
                .route_layer(from_fn(auth_login))
        )
        .nest(
            "/user",
            Router::new()
                .route("/register_contest/:id/:pwd", get(user_service::register_private_contest))
                .route("/problem_status", get(user_service::get_user_problems_status))
                .route("/new_post", post(user_service::new_post))
                .route("/update_post", post(user_service::update_post))
                .route("/del_post/:post_id", get(user_service::del_post))
                .route("/comment", post(user_service::comment))

                .route_layer(from_fn(auth_login))
        )

        .nest(
            "/contest/:id",
            Router::new()
                .route("/enter", get(user_service::enter_contest))
                .route("/info", get(user_service::get_contest_info))
                .route("/team", get(user_service::get_team))
                .route("/problems", get(user_service::get_contest_problems))
                .route("/problem/:label", get(user_service::get_cproblem))
                .route("/submit_code", post(user_service::submit_contest_code))
                .route("/cs_status/:sid", get(user_service::get_cs_status))
                .route( "/get_last_cs/:label", get(user_service::get_last_cs))
                .route("/team/:tid/csubmissions/:label", get(user_service::find_team_csubmissions))
                .route( "/show_csubmission/:sid", get(user_service::show_csubmission))
                .route( "/find_csubmissions", post(user_service::find_csbmissions))
                .route("/rank_list", get(user_service::get_rank_list))
                .route_layer(from_fn(auth_contest_role))
        )
}




pub fn config_routes() -> Router {
     Router::new()
        .nest(
            "/api",
            Router::new()
                .nest("/admin", admin_routes().route_layer(from_fn(auth_admin)))
                .nest("/", user_routes())
                .nest("/", crud_routes())
                .nest("/", account_routes())
        )
        .route_layer(from_fn(token_decode))
}