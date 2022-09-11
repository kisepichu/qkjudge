use actix_identity::Identity;
use actix_web::{get, HttpResponse, Responder};

use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Deserialize)]
struct ExecuteRequest {
    language: String,
    language_version: String,
    source: String,
    input: String,
}

#[derive(Deserialize, Default)]
#[allow(non_snake_case)]
struct CompilerApiResponse {
    output: String,
    statusCode: i32,
    memory: String,
    cpuTime: String,
}

#[derive(Serialize)]
struct ExecuteResponse {
    output: String,
    status_code: i32,
    memory: String,
    cpu_time: String,
}

#[get("/execute")]
async fn get_execute_handler(id: Identity) -> impl Responder {
    let username = id.identity().unwrap_or("".to_owned());
    if username == "" {
        return HttpResponse::Forbidden().body("not logged in".to_owned());
    }
    let client = reqwest::Client::new();
    let res = client
        .post("https://api.jdoodle.com/v1/credit-spent")
        .json(&json!({
            "clientId": std::env::var("COMPILER_API_CLIENT_ID").expect("COMPILER_API_CLIENT_ID is not set"),
            "clientSecret": std::env::var("COMPILER_API_CLIENT_SECRET").expect("COMPILER_API_CLIENT_SECRET is not set"),
        }))
        .send()
        .await
        .unwrap().text().await.unwrap();
    HttpResponse::Ok().body(res)
}
