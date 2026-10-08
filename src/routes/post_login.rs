use actix_identity::Identity;
use actix_web::{post, web, HttpResponse, Responder};
use bcrypt::verify;
use serde::Deserialize;
use std::sync::*;
use tokio::sync::Mutex;

#[derive(Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Default)]
struct User {
    username: String,
    hashed_pass: String,
}

#[post("/user/login")]
pub async fn post_login_handler(
    req: web::Json<LoginRequest>,
    pool_data: web::Data<Arc<Mutex<sqlx::Pool<sqlx::MySql>>>>,
    id: Identity,
) -> impl Responder {
    if req.password.len() > 72 || req.username.chars().count() > 30 {
        return HttpResponse::BadRequest().body("username or password is too long");
    }
    let pool = pool_data.lock().await;
    let user = sqlx::query_as!(User, "SELECT * FROM users WHERE username = ?", req.username)
        .fetch_one(&*pool)
        .await
        .unwrap_or(Default::default());
    drop(pool);
    let password = req.into_inner().password;
    let valid = web::block(move || verify(password, &user.hashed_pass))
        .await
        .ok()
        .and_then(Result::ok)
        .unwrap_or(false);
    if !valid || user.username == "" {
        return HttpResponse::Forbidden().body("username or password is wrong");
    }

    id.remember(user.username);
    HttpResponse::Ok().finish()
}
