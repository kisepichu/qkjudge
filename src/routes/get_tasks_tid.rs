use actix_identity::Identity;

use actix_web::{get, web, HttpResponse, Responder};

use serde::{Deserialize, Serialize};
use std::sync::*;
use tokio::sync::Mutex;

extern crate yaml_rust;

#[derive(Deserialize)]
struct TasksTidPath {
    task_id: i32,
}

#[derive(Serialize, Deserialize)]
struct Task {
    id: i32,
    submission_id: i32,
    input: String,
    output: String,
    expected: String,
    result: String,
    memory: String,
    cpu_time: String,
}

#[get("/tasks/{task_id}")]
async fn get_tasks_tid_handler(
    _id: Identity,
    path: web::Path<TasksTidPath>,
    pool_data: web::Data<Arc<Mutex<sqlx::Pool<sqlx::MySql>>>>,
) -> impl Responder {
    let pool = pool_data.lock().await;
    // println!("get_submissions_pid: 1");
    let task = sqlx::query_as!(Task, "SELECT * FROM tasks WHERE id=?", path.task_id)
        .fetch_one(&*pool)
        .await
        .unwrap_or(Task {
            id: 0,
            submission_id: 0,
            input: "".to_string(),
            output: "".to_string(),
            expected: "".to_string(),
            result: "".to_string(),
            memory: "".to_string(),
            cpu_time: "".to_string(),
        });

    if task.id == 0 {
        return HttpResponse::NotFound().finish();
    }

    HttpResponse::Ok().json(task)
}
