use actix_identity::Identity;
use actix_web::{get, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::prelude::*;
use std::sync::*;
use tokio::sync::Mutex;

extern crate yaml_rust;

#[derive(Default, Deserialize)]
struct Problem {
    id: i32,
    title: String,
    author: String,
    difficulty: i32,
    time_limit: String,
    memory_limit: i32,
    path: String,
    visible: i8,
}

#[derive(Serialize)]
struct GetProblemsPidResponse {
    id: i32,
    title: String,
    author: String,
    difficulty: i32,
    statement: String,
    time_limit: String,
    memory_limit: i32,
}

#[get("/problems/{problem_id}")]
async fn get_problems_pid_handler(
    _id: Identity,
    problem_id: web::Path<i32>,
    pool_data: web::Data<Arc<Mutex<sqlx::Pool<sqlx::MySql>>>>,
) -> impl Responder {
    let pool = pool_data.lock().await;
    let problem = sqlx::query_as!(
        Problem,
        "SELECT id, title, author, difficulty, time_limit, memory_limit, path, visible FROM problems WHERE id=?",
        problem_id.to_string()
    )
    .fetch_one(&*pool)
    .await
    .unwrap_or(Default::default());

    if problem.visible == 0 {
        return HttpResponse::Forbidden().body("hidden");
    }

    if problem.path == "" {
        return HttpResponse::NotFound().finish();
    }

    let statement_path = std::env::var("PROBLEMS_ROOT")
        .expect("PROBLEMS_ROOT not set")
        .replace("\r", "")
        + &problem.path
        + "/statement.md";
    let mut statement_file = match File::open(statement_path) {
        Ok(f) => f,
        Err(_e) => {
            return HttpResponse::InternalServerError().body("problemm statement file not found")
        }
    };
    let mut statement_raw = String::new();
    match statement_file.read_to_string(&mut statement_raw) {
        Ok(r) => r,
        Err(_e) => {
            return HttpResponse::InternalServerError().body("problemm configure file not found")
        }
    };

    HttpResponse::Ok().json(GetProblemsPidResponse {
        id: problem_id.into_inner(),
        title: problem.title,
        author: problem.author,
        difficulty: problem.difficulty,
        statement: statement_raw,
        time_limit: problem.time_limit,
        memory_limit: problem.memory_limit,
    })
}
