use actix_identity::Identity;
use actix_web::{post, web, HttpResponse, Responder};
use bcrypt::verify;
use serde::Deserialize;
use std::sync::*;

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
async fn post_login(
    req: web::Json<LoginRequest>,
    pool_data: web::Data<Arc<Mutex<sqlx::Pool<sqlx::MySql>>>>,
    id: Identity,
) -> impl Responder {
    let pool = pool_data.lock().unwrap();
    let user = sqlx::query_as!(User, "SELECT * FROM users WHERE username = ?", req.username)
        .fetch_one(&*pool)
        .await
        .unwrap_or(Default::default());
    let hashed_pass = user.hashed_pass;
    let valid = verify(&req.password, &hashed_pass).unwrap_or(false);
    if !valid || user.username == "" {
        return HttpResponse::Forbidden().body("username or password is wrong");
    }

    id.remember(req.username.to_owned());
    HttpResponse::Ok().finish()
}
