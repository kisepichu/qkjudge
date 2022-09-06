use actix_web::{post, web, HttpResponse, Responder};
use bcrypt::{hash, DEFAULT_COST};
use serde::Deserialize;
use std::sync::*;

#[derive(Deserialize)]
struct ProblemNewRequest {
    path: String,
}

#[post("/problem/new")]
async fn post_problem_new(
    req: web::Json<ProblemNewRequest>,
    pool_data: web::Data<Arc<Mutex<sqlx::Pool<sqlx::MySql>>>>,
) -> impl Responder {
    let pool = pool_data.lock().unwrap();
    let count = sqlx::query!(
        r#"SELECT COUNT(*) as value FROM problems WHERE path=?"#,
        req.path
    )
    .fetch_one(&*pool)
    .await
    .unwrap()
    .value;

    if count > 0 {
        return HttpResponse::Conflict().body(format!(
            "problem {} already registered",
            req.path.to_owned()
        ));
    }

    sqlx::query!("INSERT INTO problems (path) VALUES (?);", req.path)
        .execute(&*pool)
        .await
        .unwrap();

    HttpResponse::Created().finish()
}
