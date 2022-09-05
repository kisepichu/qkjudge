use actix_identity::Identity;
use actix_web::{get, post, web, HttpResponse, Responder};
use futures_util::TryFutureExt;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Deserialize)]
struct ExecuteRequest {
    language: String,
    source: String,
    input: String,
}

#[derive(Deserialize, Default)]
struct CompilerApiResponse {
    output: String,
    statusCode: i32,
    memory: String,
    cpuTime: String,
}

#[derive(Serialize)]
struct ExecuteResponse {
    output: String,
    statusCode: i32,
    memory: String,
    cpuTime: String,
}

#[post("/execute")]
async fn post_execute(req: web::Json<ExecuteRequest>, id: Identity) -> impl Responder {
    let username = id.identity().unwrap_or("".to_owned());
    if username == "" {
        return HttpResponse::Forbidden().body("not logged in".to_owned());
    }
    let client = reqwest::Client::new();
    let res = client
        .post("https://api.jdoodle.com/v1/execute")
        .json(&json!({
            "clientId": std::env::var("COMPILER_API_CLIENT_ID").expect("COMPILER_API_CLIENT_ID is not set"),
            "clientSecret": std::env::var("COMPILER_API_CLIENT_SECRET").expect("COMPILER_API_CLIENT_SECRET is not set"),
            "script": req.source,
            "language": req.language,
            "stdin": req.input
        }))
        .send()
        .await
        .unwrap().json::<CompilerApiResponse>().await.unwrap();
    HttpResponse::Ok().json(ExecuteResponse {
        output: res.output,
        statusCode: res.statusCode,
        memory: res.memory,
        cpuTime: res.cpuTime,
    })
}

#[get("/execute")]
async fn get_execute(id: Identity) -> impl Responder {
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
