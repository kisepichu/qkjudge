use actix_web::{post, web, HttpResponse, Responder};
use bcrypt::{hash, DEFAULT_COST};
use serde::Deserialize;
use std::sync::*;
use tokio::sync::Mutex;

#[derive(Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[post("/user/signup")]
pub async fn post_signup_handler(
    req: web::Json<LoginRequest>,
    pool_data: web::Data<Arc<Mutex<sqlx::Pool<sqlx::MySql>>>>,
) -> impl Responder {
    if req.username == "" || req.password == "" {
        return HttpResponse::BadRequest().body("username or password cannot be empty");
    }
    if crate::security::reserved_username(&req.username) {
        return HttpResponse::Forbidden().body("reserved username");
    }
    if req.username.chars().count() > 30 || req.password.len() > 72 {
        return HttpResponse::BadRequest().body("username or password is too long");
    }
    let req = req.into_inner();
    let password = req.password;
    let hashed_pass = match web::block(move || hash(password, DEFAULT_COST)).await {
        Ok(Ok(value)) => value,
        _ => return HttpResponse::InternalServerError().finish(),
    };
    let pool = pool_data.lock().await;
    let count = sqlx::query!(
        r#"SELECT COUNT(*) as value FROM users WHERE username=? LIMIT 1;"#,
        req.username
    )
    .fetch_one(&*pool)
    .await
    .unwrap()
    .value;

    if count > 0 {
        return HttpResponse::Conflict().body(format!("user {} already exists", req.username));
    }

    sqlx::query!(
        "INSERT INTO users (username, hashed_pass) VALUES (?, ?);",
        req.username,
        hashed_pass
    )
    .execute(&*pool)
    .await
    .unwrap();

    HttpResponse::Created().finish()
}
