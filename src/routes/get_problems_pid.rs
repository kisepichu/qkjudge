use actix_identity::Identity;
use actix_web::{get, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::prelude::*;
use std::sync::*;
use tokio::sync::Mutex;

extern crate yaml_rust;

#[derive(Default, Serialize)]
enum SolutionStatus {
    #[default]
    NotLogged,
    NotSubmitted,
    NotAccepted,
    Accepted,

    SolutionStatusNum,
}

#[derive(Deserialize)]
struct SubmissionId {
    id: i32,
}

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
    status: SolutionStatus,
    last_submission: i32,
}

#[get("/problems/{problem_id}")]
async fn get_problems_pid_handler(
    id: Identity,
    problem_id: web::Path<i32>,
    pool_data: web::Data<Arc<Mutex<sqlx::Pool<sqlx::MySql>>>>,
) -> impl Responder {
    let username = id.identity().unwrap_or("".to_owned());

    let problem: Problem;
    {
        let pool = pool_data.lock().await;
        problem = sqlx::query_as!(
            Problem,
            "SELECT id, title, author, difficulty, time_limit, memory_limit, path, visible FROM problems WHERE id=?",
            problem_id.to_string()
        )
        .fetch_one(&*pool)
        .await
        .unwrap_or(Default::default());
    }

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
        Err(_e) => return HttpResponse::InternalServerError().body("statement file not found"),
    };
    let mut statement_raw = String::new();
    match statement_file.read_to_string(&mut statement_raw) {
        Ok(r) => r,
        Err(_e) => return HttpResponse::InternalServerError().body("read_to_string failed"),
    };

    let mut status = SolutionStatus::NotLogged;
    let mut last_submission = -1;
    if username != "" {
        let pool = pool_data.lock().await;
        let submission_ac = sqlx::query_as!(
            SubmissionId,
            "SELECT id FROM submissions WHERE problem_id=? AND author=? AND result='AC' LIMIT 1",
            problem_id.to_string(),
            username,
        )
        .fetch_one(&*pool)
        .await
        .unwrap_or(SubmissionId { id: -1 });

        if submission_ac.id >= 0 {
            status = SolutionStatus::Accepted;
            last_submission = submission_ac.id;
        } else {
            let submission = sqlx::query_as!(
                SubmissionId,
                "SELECT id FROM submissions WHERE problem_id=? AND author=? LIMIT 1",
                problem_id.to_string(),
                username,
            )
            .fetch_one(&*pool)
            .await
            .unwrap_or(SubmissionId { id: -1 });
            if submission.id >= 0 {
                status = SolutionStatus::NotAccepted;
                last_submission = submission.id;
            } else {
                status = SolutionStatus::NotSubmitted;
            }
        }
    }

    HttpResponse::Ok().json(GetProblemsPidResponse {
        id: problem_id.into_inner(),
        title: problem.title,
        author: problem.author,
        difficulty: problem.difficulty,
        statement: statement_raw,
        time_limit: problem.time_limit,
        memory_limit: problem.memory_limit,
        status: status,
        last_submission: last_submission,
    })
}
