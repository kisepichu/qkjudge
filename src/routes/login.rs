use actix_identity::{Identity};

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
#[allow(non_snake_case)]
struct User {
    Username: String,
    HashedPass: String,
}

#[post("/login")]
async fn post_login(
    req: web::Json<LoginRequest>,
    pool_data: web::Data<Arc<Mutex<sqlx::Pool<sqlx::MySql>>>>,
    id: Identity,
) -> impl Responder {
    let pool = pool_data.lock().unwrap();
    let user = sqlx::query_as!(User, "SELECT * FROM users WHERE Username = ?", req.username)
        .fetch_one(&*pool)
        .await
        .unwrap_or(Default::default());
    let hashed_pass = user.HashedPass;
    let valid = verify(&req.password, &hashed_pass).unwrap_or(false);
    if !valid || user.Username == "" {
        return HttpResponse::Forbidden().body("username or password is wrong");
    }

    id.remember(req.username.to_owned());
    HttpResponse::Ok().finish()
}
