use actix_identity::Identity;
use actix_web::cookie::time::PrimitiveDateTime;
use actix_web::{get, web, HttpResponse, Responder};

use serde::{Deserialize, Serialize};
use std::sync::*;

extern crate yaml_rust;

#[derive(Deserialize)]
struct SubmissionsSidPath {
    submission_id: i32,
}

#[derive(Serialize, Deserialize)]
struct Submission {
    id: i32,
    date: PrimitiveDateTime,
    author: String,
    problem_id: i32,
    testcase_num: i32,
    result: String,
    language: String,
    language_version: String,
    source: String,
}

#[derive(Serialize)]
struct GetSubmissionsSidResponse {
    id: i32,
    date: String,
    author: String,
    problem_id: i32,
    testcase_num: i32,
    progress_num: i32,
    result: String,
    language: String,
    language_version: String,
    source: String,
}

#[get("/submissions/{submission_id}")]
async fn get_submissions_sid(
    _id: Identity,
    path: web::Path<SubmissionsSidPath>,
    pool_data: web::Data<Arc<Mutex<sqlx::Pool<sqlx::MySql>>>>,
) -> impl Responder {
    // println!("get_submissions_pid: 0");
    // ログインしていなかったら弾く
    // let username = id.identity().unwrap_or("".to_owned());
    // if username == "" {
    //     return HttpResponse::Forbidden().body("not logged in".to_owned());
    // }
    let _username = "tqk";

    let pool = pool_data.lock().unwrap();
    // println!("get_submissions_pid: 1");
    let submission = sqlx::query_as!(
        Submission,
        "SELECT * FROM submissions WHERE id=?",
        path.submission_id
    )
    .fetch_one(&*pool)
    .await
    .unwrap_or(Submission {
        id: 0,
        date: PrimitiveDateTime::MIN,
        author: "".to_string(),
        problem_id: 0,
        testcase_num: 0,
        result: "".to_string(),
        language: "".to_string(),
        language_version: "".to_string(),
        source: "".to_string(),
    });

    if submission.id == 0 {
        return HttpResponse::NotFound().finish();
    }

    // println!("get_submissions_pid: 2");
    let progress_num = sqlx::query!(
        r#"SELECT COUNT(*) as value FROM tasks WHERE submission_id=?"#,
        submission.id
    )
    .fetch_one(&*pool)
    .await
    .unwrap()
    .value as i32;
    // println!("get_submissions_pid: 3");

    HttpResponse::Ok().json(GetSubmissionsSidResponse {
        id: submission.id,
        date: submission.date.to_string(),
        author: submission.author,
        problem_id: submission.problem_id,
        testcase_num: submission.testcase_num,
        progress_num: progress_num,
        result: submission.result,
        language: submission.language,
        language_version: submission.language_version,
        source: submission.source,
    })
}
