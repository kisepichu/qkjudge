use actix_identity::Identity;
use actix_web::{get, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::prelude::*;
use std::sync::*;
use yaml_rust::YamlLoader;
extern crate yaml_rust;

#[derive(Default, Deserialize)]
struct ProblemLocation {
    id: i32,
    path: String,
}

#[derive(Serialize)]
struct GetProblemsPidResponse {
    problem_id: i32,
    title: String,
    author: String,
    difficulty: i64,
    statement: String,
}

#[get("/problems/{problem_id}")]
async fn get_problems_pid_handler(
    _id: Identity,
    problem_id: web::Path<i32>,
    pool_data: web::Data<Arc<Mutex<sqlx::Pool<sqlx::MySql>>>>,
) -> impl Responder {
    let pool = pool_data.lock().unwrap();
    let problem_path = sqlx::query_as!(
        ProblemLocation,
        "SELECT * FROM problems WHERE id=?",
        problem_id.to_string()
    )
    .fetch_one(&*pool)
    .await
    .unwrap_or(Default::default())
    .path;

    let info_path = std::env::var("PROBLEMS_ROOT")
        .expect("PROBLEMS_ROOT not set")
        .replace("\r", "")
        + &problem_path
        + "/problem.yaml";
    println!("{:?}", info_path);
    let mut info_file = File::open(info_path).expect("file not found");
    let mut info_raw = String::new();
    info_file
        .read_to_string(&mut info_raw)
        .expect("something went wrong reading the file");
    let docs = YamlLoader::load_from_str(&info_raw).unwrap();
    let info = &docs[0];

    let statement_path = std::env::var("PROBLEMS_ROOT")
        .expect("PROBLEMS_ROOT not set")
        .replace("\r", "")
        + &problem_path
        + "/statement.md";
    let mut statement_file = File::open(statement_path).expect("file not found");
    let mut statement_raw = String::new();
    statement_file
        .read_to_string(&mut statement_raw)
        .expect("something went wrong reading the file");

    HttpResponse::Ok().json(GetProblemsPidResponse {
        problem_id: problem_id.into_inner(),
        title: info["title"].as_str().unwrap().to_string(),
        author: info["author"].as_str().unwrap().to_string(),
        difficulty: info["difficulty"].as_i64().unwrap(),
        statement: statement_raw,
    })
}
