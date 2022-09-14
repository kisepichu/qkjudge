use actix_identity::Identity;
use actix_web::{get, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

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

#[derive(Serialize, Deserialize)]
struct Problem {
    id: i32,
    title: String,
    author: String,
    difficulty: i32,
}

#[derive(Serialize)]
struct ProblemInResponse {
    id: i32,
    title: String,
    author: String,
    difficulty: i32,
    status: SolutionStatus,
    last_submission: i32,
}

#[derive(Serialize)]
struct GetProblemsResponse {
    problems: Vec<ProblemInResponse>,
}

#[get("/problems")]
async fn get_problems_handler(
    id: Identity,
    pool_data: web::Data<Arc<Mutex<sqlx::Pool<sqlx::MySql>>>>,
) -> impl Responder {
    let username = id.identity().unwrap_or("".to_owned());

    let pool = pool_data.lock().await;
    let problems = sqlx::query_as!(
        Problem,
        "SELECT id, title, author, difficulty FROM problems WHERE visible=true ORDER BY difficulty, title;"
    )
    .fetch_all(&*pool)
    .await
    .unwrap_or(vec![]);

    let mut res: Vec<ProblemInResponse> = vec![];
    for problem in problems {
        let mut status = SolutionStatus::NotLogged;
        let mut last_submission = -1;
        if username != "" {
            let pool = pool_data.lock().await;
            let submission_ac = sqlx::query_as!(
                SubmissionId,
                "SELECT id FROM submissions WHERE problem_id=? AND author=? AND result='AC' LIMIT 1",
                problem.id.to_string(),
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
                    problem.id.to_string(),
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

        res.push(ProblemInResponse {
            id: problem.id,
            title: problem.title.clone(),
            author: problem.author,
            difficulty: problem.difficulty,
            status: status,
            last_submission: last_submission,
        });
    }

    HttpResponse::Ok().json(GetProblemsResponse { problems: res })
}
