use actix_identity::Identity;
use actix_web::{get, web, HttpResponse, Responder};
use serde::Deserialize;

use crate::legacy_store;

#[derive(Deserialize)]
struct LegacyTasksTidPath {
    task_id: i32,
}

#[get("/legacy/tasks/{task_id}")]
async fn get_legacy_tasks_tid_handler(
    _id: Identity,
    path: web::Path<LegacyTasksTidPath>,
) -> impl Responder {
    let store = legacy_store::global();
    match store.task(path.task_id) {
        Some(t) => HttpResponse::Ok().json(t),
        None => HttpResponse::NotFound().finish(),
    }
}
