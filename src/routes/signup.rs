use actix_cors::Cors;
use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::cookie::SameSite;
use actix_web::web::Data;
use actix_web::{
    delete, get, http, middleware, options, post, web, App, HttpRequest, HttpResponse, HttpServer,
    Responder,
};
use bcrypt::{hash, verify, DEFAULT_COST};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::*;

#[derive(Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Default)]
#[allow(non_snake_case)]
struct User {
    Username: String,
    HashedPass: String,
}

#[post("/signup")]
async fn post_signup(
    req: web::Json<LoginRequest>,
    pool_data: web::Data<Arc<Mutex<sqlx::Pool<sqlx::MySql>>>>,
) -> impl Responder {
    let pool = pool_data.lock().unwrap();
    if req.username == "" || req.password == "" {
        return HttpResponse::BadRequest().body("username or password cannot be empty");
    }
    let hashed_pass = hash(&req.password, DEFAULT_COST).unwrap();
    let count = sqlx::query!(
        r#"SELECT COUNT(*) as value FROM users WHERE Username=?"#,
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
        "INSERT INTO users (Username, HashedPass) VALUES (?, ?);",
        req.username,
        hashed_pass
    )
    .execute(&*pool)
    .await
    .unwrap();

    HttpResponse::Created().finish()
}
