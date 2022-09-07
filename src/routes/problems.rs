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
struct Problem {
    problem_id: i32,
    title: String,
    author: String,
    difficulty: i64,
}

#[derive(Serialize)]
struct GetProblemsResponse {
    problems: Vec<Problem>,
}

#[get("/problems")]
async fn get_problems(
    _id: Identity,
    pool_data: web::Data<Arc<Mutex<sqlx::Pool<sqlx::MySql>>>>,
) -> impl Responder {
    let pool = pool_data.lock().unwrap();
    let problems = sqlx::query_as!(ProblemLocation, "SELECT * FROM problems")
        .fetch_all(&*pool)
        .await
        .unwrap_or(vec![]);
    let mut ret: Vec<Problem> = vec![];
    for problem in problems.iter() {
        let path = std::env::var("PROBLEMS_ROOT")
            .expect("PROBLEMS_ROOT not set")
            .replace("\r", "")
            + &problem.path
            + "/problem.yaml";
        println!("{:?}", path);
        let mut file = File::open(path).expect("file not found");
        let mut raw = String::new();
        file.read_to_string(&mut raw)
            .expect("something went wrong reading the file");
        let docs = YamlLoader::load_from_str(&raw).unwrap();
        let doc = &docs[0];

        ret.push(Problem {
            problem_id: problem.id,
            title: doc["title"].as_str().unwrap().to_string(),
            author: doc["author"].as_str().unwrap().to_string(),
            difficulty: doc["difficulty"].as_i64().unwrap(),
        });
    }
    HttpResponse::Ok().json(GetProblemsResponse { problems: ret })
}
