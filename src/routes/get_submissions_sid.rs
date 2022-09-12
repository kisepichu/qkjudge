use actix_identity::Identity;
use actix_web::cookie::time::PrimitiveDateTime;
use actix_web::{get, web, HttpResponse, Responder};

use serde::{Deserialize, Serialize};
use std::sync::*;
use tokio::sync::Mutex;

use crate::languages::LANGUAGES;

extern crate yaml_rust;

#[derive(Deserialize)]
struct SubmissionsSidPath {
    submission_id: i32,
}

#[derive(Deserialize, Serialize)]
struct TaskSummary {
    id: i32,
    result: String,
}

#[derive(Serialize, Deserialize)]
struct Submission {
    id: i32,
    date: PrimitiveDateTime,
    author: String,
    problem_id: i32,
    testcase_num: i32,
    result: String,
    language_id: i32,
    source: String,
}

#[derive(Serialize)]
struct GetSubmissionsSidResponse {
    id: i32,
    date: String,
    author: String,
    problem_id: i32,
    testcase_num: i32,
    tasks: Vec<TaskSummary>,
    result: String,
    language_id: i32,
    source: String,
}

#[get("/submissions/{submission_id}")]
async fn get_submissions_sid_handler(
    _id: Identity,
    path: web::Path<SubmissionsSidPath>,
    pool_data: web::Data<Arc<Mutex<sqlx::Pool<sqlx::MySql>>>>,
) -> impl Responder {
    // println!("get_submissions_pid: 1");
    let submission: Submission;
    {
        let pool = pool_data.lock().await;
        submission = sqlx::query_as!(
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
            language_id: -1,
            source: "".to_string(),
        });
    }

    if submission.id == 0 {
        return HttpResponse::NotFound().finish();
    }

    let tasks: Vec<TaskSummary>;
    {
        let pool = pool_data.lock().await;

        tasks = sqlx::query_as!(
            TaskSummary,
            "SELECT id, result FROM tasks WHERE submission_id=?",
            submission.id
        )
        .fetch_all(&*pool)
        .await
        .unwrap();
    }
    let progress_num = tasks.len();

    let result = if (submission.result == "WJ") {
        format!("WJ {}/{}", progress_num, submission.testcase_num)
    } else {
        submission.result
    };

    HttpResponse::Ok().json(GetSubmissionsSidResponse {
        id: submission.id,
        date: submission.date.to_string(),
        author: submission.author,
        problem_id: submission.problem_id,
        testcase_num: submission.testcase_num,
        tasks: tasks,
        result: result,
        language_id: submission.language_id,
        source: submission.source,
    })
}
