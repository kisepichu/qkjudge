use actix_cors::Cors;
use actix_identity::Identity;
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

#[options("{_}")]
async fn preflight() -> impl Responder {
    HttpResponse::Ok().finish()
}

#[get("/")]
async fn index(req: HttpRequest) -> impl Responder {
    println!("Request: {req:?}");
    HttpResponse::Ok().body("Hello, World! frontend: https://tqk.blue/reactpractice/")
}

#[get("/ping")]
async fn ping() -> impl Responder {
    HttpResponse::Ok().body("pong")
}

#[derive(Default, Serialize)]
#[allow(non_snake_case)]
struct City {
    ID: i32,
    Name: String,
    CountryCode: String,
    District: String,
    Population: i32,
}

#[get("/dbtest")]
async fn dbtest(
    pool_data: web::Data<Arc<Mutex<sqlx::Pool<sqlx::MySql>>>>,
    id: Identity,
) -> impl Responder {
    let username = id.identity().unwrap_or("".to_string());
    if username == "" {
        return HttpResponse::Forbidden().body("login required");
    }

    println!("{:?}", pool_data);
    let pool = pool_data.lock().unwrap();
    let mut ret: Vec<String> = vec![];

    let row_tokyo = sqlx::query_as!(City, r#"SELECT * FROM city WHERE Name='Tokyo'"#)
        .fetch_one(&*pool)
        .await
        .unwrap_or(Default::default());
    ret.push(format!("東京都のid: {}\n", row_tokyo.ID));
    println!("{}", row_tokyo.Name);

    let rows_city_in_japan = sqlx::query_as!(City, r#"SELECT * FROM city WHERE CountryCode='JPN'"#)
        .fetch_all(&*pool)
        .await
        .unwrap_or(Default::default());
    ret.push("日本の都市一覧:".to_string());
    for row in rows_city_in_japan.iter() {
        ret.push(format!("名前: {}, 人口: {}", row.Name, row.Population));
    }

    HttpResponse::Ok().body(ret.join("\n"))
}

#[get("/hello")]
async fn hello(id: Identity) -> impl Responder {
    HttpResponse::Ok().body(format!(
        "Hello, {}!",
        id.identity().unwrap_or("guest".to_owned())
    ))
}

#[get("/whoami")]
async fn whoami(id: Identity) -> impl Responder {
    let username = id.identity().unwrap_or("".to_owned());
    if username == "" {
        return HttpResponse::Forbidden().body("not logged in".to_owned());
    }
    HttpResponse::Ok().body(username)
}

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
async fn signup(
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

#[post("/login")]
async fn login(
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

#[delete("/logout")]
async fn logout(id: Identity) -> impl Responder {
    id.forget();
    HttpResponse::Ok().finish()
}

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
            .service(preflight)
            .service(index)
            .service(signup)
            .service(login)
            .service(logout)
            .service(ping)
            .service(hello)
            .service(dbtest)
            .service(whoami)
    })
    .bind((address, 8080))?
    .run()
    .await
}
