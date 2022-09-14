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
async fn post_signup_handler(
    req: web::Json<LoginRequest>,
    pool_data: web::Data<Arc<Mutex<sqlx::Pool<sqlx::MySql>>>>,
) -> impl Responder {
    let pool = pool_data.lock().await;
    if req.username == "" || req.password == "" {
        return HttpResponse::BadRequest().body("username or password cannot be empty");
    }
    let hashed_pass = hash(&req.password, DEFAULT_COST).unwrap();
    let count = sqlx::query!(
        r#"SELECT COUNT(*) as value FROM users WHERE username=? LIMIT 1;"#,
        req.username
    )
    .fetch_one(&*pool)
    .await
    .unwrap()
    .value;

    if count > 0 {
        return HttpResponse::Conflict()
            .body(format!("user {} already exists", req.username.to_owned()));
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
