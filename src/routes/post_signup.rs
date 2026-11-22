use super::ADMIN_USERNAME;
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
    if req.username.is_empty() || req.password.is_empty() {
        return HttpResponse::BadRequest().body("username or password cannot be empty");
    }
    // admin は put_submissions_sid の特権名。誰でも先に登録できると他人の提出を再ジャッジできる。
    // UI は 409 を「その名前は使われている」と表示するので、予約名も 409 で返す。
    if is_reserved_username(&req.username) {
        return HttpResponse::Conflict().body(format!("username {} is reserved", req.username));
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

fn is_reserved_username(username: &str) -> bool {
    username.eq_ignore_ascii_case(ADMIN_USERNAME)
}

#[cfg(test)]
mod test {
    use super::is_reserved_username;

    #[test]
    fn admin_is_reserved_in_any_ascii_case() {
        // users.username の照合順序は大文字小文字を区別しないので、"ADMIN" で登録されると
        // "admin" 自身を後から登録できなくなる。大文字小文字違いもまとめて予約する。
        for name in ["admin", "Admin", "ADMIN", "aDmIn"] {
            assert!(is_reserved_username(name), "{name}");
        }
    }

    #[test]
    fn names_merely_containing_admin_are_not_reserved() {
        for name in ["tqk", "admin2", "administrator", "myadmin"] {
            assert!(!is_reserved_username(name), "{name}");
        }
    }
}
