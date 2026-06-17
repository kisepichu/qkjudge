use actix_web::{get, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

use crate::legacy_store;

#[derive(Deserialize)]
struct LegacySubmissionsSidPath {
    submission_id: i32,
}

#[derive(Serialize)]
struct TaskSummary {
    id: i32,
    result: String,
}

#[derive(Serialize)]
struct GetLegacySubmissionsSidResponse {
    id: i32,
    date: String,
    author: String,
    problem_id: i32,
    problem_title: String,
    testcase_num: i32,
    tasks: Vec<TaskSummary>,
    result: String,
    language_id: i32,
    source: String,
}

#[get("/legacy/submissions/{submission_id}")]
async fn get_legacy_submissions_sid_handler(
    path: web::Path<LegacySubmissionsSidPath>,
) -> impl Responder {
    let store = legacy_store::global();
    let Some(s) = store.submission(path.submission_id) else {
        return HttpResponse::NotFound().finish();
    };

    HttpResponse::Ok().json(GetLegacySubmissionsSidResponse {
        id: s.id,
        date: s.date.clone(),
        author: s.author.clone(),
        problem_id: s.problem_id,
        problem_title: s.problem_title.clone(),
        testcase_num: s.testcase_num,
        tasks: s
            .tasks
            .iter()
            .map(|t| TaskSummary {
                id: t.id,
                result: t.result.clone(),
            })
            .collect(),
        result: s.result.clone(),
        language_id: s.language_id,
        source: s.source.clone(),
    })
}
