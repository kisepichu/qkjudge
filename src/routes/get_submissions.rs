use actix_identity::Identity;
use actix_web::cookie::time::PrimitiveDateTime;
use actix_web::{get, web, HttpResponse, Responder};

use serde::{Deserialize, Serialize};
use std::sync::*;
use tokio::sync::Mutex;

extern crate yaml_rust;

#[derive(Deserialize)]
struct SubmissionsQuery {
    page: Option<i32>,
}

#[derive(Deserialize)]
struct SubmissionSummary {
    id: i32,
    date: PrimitiveDateTime,
    author: String,
    problem_id: i32,
    problem_title: String,
    result: String,
    language_id: i32,
}

#[derive(Serialize)]
struct SubmissionSummaryInResponse {
    id: i32,
    date: String,
    author: String,
    problem_id: i32,
    problem_title: String,
    result: String,
    language_id: i32,
}

#[derive(Serialize)]
struct Problem {
    problem_id: i32,
    title: String,
    author: String,
    difficulty: i64,
}

#[derive(Serialize)]
struct GetSubmissionsResponse {
    pages_number: i32,
    submissions: Vec<SubmissionSummaryInResponse>,
}

#[get("/submissions")]
async fn get_submissions_handler(
    _id: Identity,
    query: web::Query<SubmissionsQuery>,
    pool_data: web::Data<Arc<Mutex<sqlx::Pool<sqlx::MySql>>>>,
) -> impl Responder {
    let submissions_in_page = 10;
    let page = query.page.unwrap_or(1);
    if page <= 0 {
        return HttpResponse::BadRequest().body("submissions_page must be positive");
    }

    let pool = pool_data.lock().await;
    let submissions = sqlx::query_as!(
        SubmissionSummary,
        r#"SELECT submissions.id, date, submissions.author, problem_id, title AS problem_title, result, language_id
        FROM submissions INNER JOIN problems ON submissions.problem_id=problems.id
        ORDER BY id DESC LIMIT ?, ?;"#,
        submissions_in_page * (page - 1),
        submissions_in_page
    )
    .fetch_all(&*pool)
    .await
    .unwrap_or(vec![]);

    let submissions_number = sqlx::query!("SELECT COUNT(*) as value FROM submissions;")
        .fetch_one(&*pool)
        .await
        .unwrap()
        .value as i32;
    let pages_number = (submissions_number + submissions_in_page - 1) / submissions_in_page;

    HttpResponse::Ok().json(GetSubmissionsResponse {
        pages_number: pages_number,
        submissions: submissions
            .iter()
            .map(|s| SubmissionSummaryInResponse {
                id: s.id.clone(),
                date: s.date.to_string(),
                author: s.author.clone(),
                problem_id: s.problem_id.clone(),
                problem_title: s.problem_title.clone(),
                result: s.result.clone(),
                language_id: s.language_id,
            })
            .collect::<Vec<SubmissionSummaryInResponse>>(),
    })
}
