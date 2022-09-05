use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::cookie::SameSite;
use actix_web::web::Data;
use actix_web::{middleware, App, HttpServer};
use rand::Rng;
use std::env;
use std::sync::*;

mod routes;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env::set_var("RUST_LOG", "actix_web=info");
    std::env::set_var("RUST_BACKTRACE", "1");
    env_logger::init();
    let address = match env::var("NARO3_ADDRESS") {
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
    let private_key = rand::thread_rng().gen::<[u8; 32]>();
    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(pool_data.clone()))
            .wrap(
                middleware::DefaultHeaders::new()
                    .add(("Access-Control-Allow-Origin", "https://tqk.blue"))
                    .add(("Access-Control-Allow-Credentials", "true"))
                    .add(("Access-Control-Allow-Methods", "GET, POST, DELETE, OPTIONS"))
                    .add(("Access-Control-Allow-Headers", "Content-Type")),
            )
            .wrap(IdentityService::new(
                CookieIdentityPolicy::new(&private_key)
                    .name("auth")
                    .same_site(SameSite::None)
                    .secure(true),
            ))
            .wrap(middleware::Logger::default())
            .service(routes::options_any)
            .service(routes::get_index)
            .service(routes::post_signup)
            .service(routes::post_login)
            .service(routes::delete_logout)
            .service(routes::get_ping)
            .service(routes::get_hello)
            .service(routes::get_whoami)
    })
    .bind((address, 8080))?
    .run()
    .await
}
