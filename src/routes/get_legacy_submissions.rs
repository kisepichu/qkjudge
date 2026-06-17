use actix_identity::Identity;
use actix_web::{get, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

use crate::legacy_store;

#[derive(Deserialize)]
struct LegacySubmissionsQuery {
    page: Option<i32>,
}

#[derive(Serialize)]
struct LegacySubmissionSummaryInResponse {
    id: i32,
    date: String,
    author: String,
    problem_id: i32,
    problem_title: String,
    result: String,
    language_id: i32,
}

#[derive(Serialize)]
struct GetLegacySubmissionsResponse {
    pages_number: i32,
    submissions: Vec<LegacySubmissionSummaryInResponse>,
}

#[get("/legacy/submissions")]
async fn get_legacy_submissions_handler(
    _id: Identity,
    query: web::Query<LegacySubmissionsQuery>,
) -> impl Responder {
    let page = query.page.unwrap_or(1);
    if page <= 0 {
        return HttpResponse::BadRequest().body("submissions_page must be positive");
    }

    let store = legacy_store::global();
    let submissions = store
        .page(page, legacy_store::PER_PAGE)
        .iter()
        .map(|s| LegacySubmissionSummaryInResponse {
            id: s.id,
            date: s.date.clone(),
            author: s.author.clone(),
            problem_id: s.problem_id,
            problem_title: s.problem_title.clone(),
            result: s.result.clone(),
            language_id: s.language_id,
        })
        .collect();

    HttpResponse::Ok().json(GetLegacySubmissionsResponse {
        pages_number: store.pages_number(legacy_store::PER_PAGE),
        submissions,
    })
}
