use actix_identity::Identity;
use actix_web::cookie::time::PrimitiveDateTime;
use actix_web::{get, web, HttpResponse, Responder};

use serde::{Deserialize, Serialize};
use std::sync::*;

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
    result: String,
    language: String,
}

#[derive(Serialize)]
struct SubmissionSummaryInResponse {
    id: i32,
    date: String,
    author: String,
    problem_id: i32,
    result: String,
    language: String,
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
    submissions: Vec<SubmissionSummaryInResponse>,
}

#[get("/submissions")]
async fn get_submissions(
    _id: Identity,
    query: web::Query<SubmissionsQuery>,
    pool_data: web::Data<Arc<Mutex<sqlx::Pool<sqlx::MySql>>>>,
) -> impl Responder {
    let submissions_in_page = 10;
    let page = query.page.unwrap_or(1);
    // ログインしていなかったら弾く
    // let username = id.identity().unwrap_or("".to_owned());
    // if username == "" {
    //     return HttpResponse::Forbidden().body("not logged in".to_owned());
    // }
    let _username = "tqk";

    if page <= 0 {
        return HttpResponse::BadRequest().body("submissions_page must be positive");
    }

    let pool = pool_data.lock().unwrap();
    let submissions = sqlx::query_as!(
        SubmissionSummary,
        "SELECT id, date, author, problem_id, result, language FROM submissions LIMIT ?, ?;",
        submissions_in_page * (page - 1),
        submissions_in_page
    )
    .fetch_all(&*pool)
    .await
    .unwrap_or(vec![]);

    HttpResponse::Ok().json(GetSubmissionsResponse {
        submissions: submissions
            .iter()
            .map(|s| SubmissionSummaryInResponse {
                id: s.id.clone(),
                date: s.date.to_string(),
                author: s.author.clone(),
                problem_id: s.problem_id.clone(),
                result: s.result.clone(),
                language: s.language.clone(),
            })
            .collect::<Vec<SubmissionSummaryInResponse>>(),
    })
}
