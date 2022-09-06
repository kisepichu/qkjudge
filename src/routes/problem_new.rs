use actix_web::{post, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::fs::File;
use std::io::prelude::*;
use std::sync::*;

#[derive(Deserialize)]
struct ProblemNewRequest {
    path: String,
}

#[derive(Serialize)]
struct ProblemNewResponse {
    problem_id: i32,
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

    let path = std::env::var("PROBLEMS_ROOT")
        .expect("PROBLEMS_ROOT not set")
        .replace("\r", "")
        + &req.path
        + "/problem.yaml";
    println!("{:?}", path);
    match File::open(path) {
        Ok(_file) => true,
        Err(e) => {
            return HttpResponse::BadRequest().body(format!(
                "upload to problems repository before register. err: {}",
                e
            ));
        }
    };

    let ret = sqlx::query!(
        "INSERT INTO problems (path) VALUES (?) RETURNING id;",
        req.path
    )
    .fetch_one(&*pool)
    .await
    .unwrap()
    .get::<i32, _>(0);

    HttpResponse::Created().json(ProblemNewResponse { problem_id: ret })
}
