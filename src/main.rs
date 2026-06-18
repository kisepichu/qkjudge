use actix_cors::Cors;
use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_rt::{self, Arbiter};
use actix_web::cookie::SameSite;
use actix_web::http::header;
use actix_web::web::Data;
use actix_web::{middleware, App, HttpServer};
use rand::Rng;
use std::env;
use std::sync::*;
use tokio::sync::Mutex;

mod languages;
mod legacy_store;
mod routes;

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    env::set_var("RUST_LOG", "actix_web=info");
    std::env::set_var("RUST_BACKTRACE", "1");
    env_logger::init();
    let address = match env::var("QKJUDGE_ADDRESS") {
        Ok(val) => val,
        Err(_e) => "localhost".to_string(),
    };
    let database = env::var("MARIADB_DATABASE").expect("MARIADB_DATABASE is not set");
    let user = env::var("MARIADB_USERNAME").expect("MARIADB_USERNAME is not set");
    let password = env::var("MARIADB_PASSWORD").expect("MARIADB_PASSWORD is not set");
    let port = env::var("DB_PORT").unwrap_or("3306".to_string());
    let host = env::var("MARIADB_HOSTNAME").unwrap_or("localhost".to_string());

    // mysql://user:pass@127.0.0.1:3306/db_name
    let database_url = format!(
        "mysql://{}:{}@{}:{}/{}",
        user, password, host, port, database
    );

    let pool = sqlx::mysql::MySqlPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .unwrap();

    let pool_data = Arc::new(Mutex::new(pool));

    let arbiter = Arbiter::new();
    let arbiter_data = Arc::new(Mutex::new(arbiter));

    // CORS 許可オリジン: CSV で複数オリジン / ワイルドカードサフィックス (*) を指定可。
    // 例: "https://qkjudge-stg.kisen.one,https://*.qkjudge-ui.pages.dev"
    // Access-Control-Allow-Credentials: true と両立させるためベア "*" は不可。
    // マッチした origin のみ ACAO を返し、wildcard 反射はしない。
    let cors_allow_origin_raw = env::var("CORS_ALLOW_ORIGIN")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "http://localhost:3000".to_string());
    let cors_origins: Arc<Vec<String>> = Arc::new(
        cors_allow_origin_raw
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .inspect(|s| {
                assert!(
                    s != "*",
                    "CORS_ALLOW_ORIGIN entries must not be bare '*' \
                     (wildcard is incompatible with Access-Control-Allow-Credentials: true)"
                );
            })
            .collect(),
    );
    assert!(
        !cors_origins.is_empty(),
        "CORS_ALLOW_ORIGIN must not be empty"
    );
    log::info!("CORS allowed origins: {:?}", *cors_origins);

    // cookie 署名鍵: SESSION_KEY (64 hex = 32 byte) を渡すと再起動でログイン状態を維持できる。
    // 未設定時のみランダム生成 (開発用。再起動で全員ログアウトする)。
    let private_key: Vec<u8> = match env::var("SESSION_KEY") {
        // 空文字 (`.env` に `SESSION_KEY=` のまま等) も「未設定」とみなしフォールバックする
        // (空のまま hex decode → 長さ assert で起動時 panic するのを避ける)。
        Ok(hex_key) if !hex_key.trim().is_empty() => {
            let key = hex::decode(hex_key.trim()).expect("SESSION_KEY must be valid hex");
            assert!(
                key.len() >= 32,
                "SESSION_KEY must decode to at least 32 bytes (64 hex chars)"
            );
            key
        }
        _ => {
            log::warn!(
                "SESSION_KEY not set; generating a random cookie key (sessions reset on restart)"
            );
            rand::thread_rng().gen::<[u8; 32]>().to_vec()
        }
    };

    // Secure cookie はブラウザが plain HTTP では送受信しないため、http の手元 compose では
    // ログインできない。COOKIE_SECURE で切替可能にする (未設定/空はデフォルト true)。
    // 許可値は true/1/false/0 のみ。typo 等の想定外値は黙って true 扱いにせず起動時に落とす。
    let cookie_secure = match env::var("COOKIE_SECURE") {
        Ok(v) if !v.trim().is_empty() => match v.trim().to_ascii_lowercase().as_str() {
            "true" | "1" => true,
            "false" | "0" => false,
            other => panic!("COOKIE_SECURE must be one of true/1/false/0 (got {other:?})"),
        },
        _ => true,
    };

    // Fail fast at startup if the embedded legacy snapshot fails to deserialize,
    // instead of letting the first /legacy/* request panic the worker.
    let legacy_total = legacy_store::global().total_count();
    log::info!("legacy snapshot loaded ({legacy_total} submissions)");

    HttpServer::new(move || {
        let cors_origins_fn = cors_origins.clone();
        let cors = Cors::default()
            .allowed_origin_fn(move |origin, _req_head| {
                let origin_str = match origin.to_str() {
                    Ok(s) => s,
                    Err(_) => return false,
                };
                cors_origins_fn.iter().any(|pattern| {
                    if let Some(idx) = pattern.find('*') {
                        let prefix = &pattern[..idx];
                        let suffix = &pattern[idx + 1..];
                        origin_str.starts_with(prefix) && origin_str.ends_with(suffix)
                    } else {
                        origin_str == pattern.as_str()
                    }
                })
            })
            .allowed_methods(vec!["GET", "POST", "DELETE", "PUT", "OPTIONS"])
            .allowed_header(header::CONTENT_TYPE)
            .supports_credentials();

        App::new()
            .app_data(Data::new(pool_data.clone()))
            .app_data(Data::new(arbiter_data.clone()))
            .wrap(cors)
            .wrap(IdentityService::new(
                CookieIdentityPolicy::new(&private_key)
                    .name("auth")
                    .same_site(SameSite::Lax)
                    .secure(cookie_secure),
            ))
            .wrap(middleware::Logger::default())
            .service(routes::options_0_handler)
            .service(routes::options_1_handler)
            .service(routes::options_2_handler)
            .service(routes::get_index_handler)
            .service(routes::post_signup_handler)
            .service(routes::post_login_handler)
            .service(routes::post_logout_handler)
            .service(routes::get_ping_handler)
            .service(routes::get_hello_handler)
            .service(routes::get_whoami_handler)
            .service(routes::get_execute_handler)
            .service(routes::post_execute_handler)
            .service(routes::get_problems_handler)
            .service(routes::post_problems_handler)
            .service(routes::get_problems_pid_handler)
            .service(routes::post_submit_handler)
            .service(routes::get_submissions_sid_handler)
            .service(routes::get_submissions_handler)
            .service(routes::get_tasks_tid_handler)
            .service(routes::get_legacy_submissions_handler)
            .service(routes::get_legacy_submissions_sid_handler)
            .service(routes::get_legacy_tasks_tid_handler)
            .service(routes::put_submissions_sid_handler)
            .service(routes::post_fetch_problems_handler)
    })
    .bind((address, 8080))?
    .run()
    .await
}
