use actix_identity::Identity;
use actix_web::{get, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

use std::sync::*;
use tokio::sync::Mutex;

extern crate yaml_rust;

#[derive(Serialize, Deserialize)]
struct Problem {
    id: i32,
    title: String,
    author: String,
    difficulty: i32,
}

#[derive(Serialize)]
struct GetProblemsResponse {
    problems: Vec<Problem>,
}

#[get("/problems")]
async fn get_problems_handler(
    _id: Identity,
    pool_data: web::Data<Arc<Mutex<sqlx::Pool<sqlx::MySql>>>>,
) -> impl Responder {
    let pool = pool_data.lock().await;
    let problems = sqlx::query_as!(
        Problem,
        "SELECT id, title, author, difficulty FROM problems WHERE visible=true ORDER BY difficulty, title;"
    )
    .fetch_all(&*pool)
    .await
    .unwrap_or(vec![]);

    HttpResponse::Ok().json(GetProblemsResponse { problems: problems })
}
