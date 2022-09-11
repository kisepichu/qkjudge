use actix_identity::Identity;
use actix_web::{post, web, HttpResponse, Responder};

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::languages::LANGUAGES;

#[derive(Deserialize)]
struct ExecuteRequest {
    language_id: i32,
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

#[post("/execute")]
async fn post_execute_handler(req: web::Json<ExecuteRequest>, _id: Identity) -> impl Responder {
    // let username = id.identity().unwrap_or("".to_owned());
    // if username == "" {
    //     return HttpResponse::Forbidden().body("not logged in".to_owned());
    // }
    let client = reqwest::Client::new();
    let res = client
        .post("https://api.jdoodle.com/v1/execute")
        .json(&json!({
            "clientId": std::env::var("COMPILER_API_CLIENT_ID").expect("COMPILER_API_CLIENT_ID is not set"),
            "clientSecret": std::env::var("COMPILER_API_CLIENT_SECRET").expect("COMPILER_API_CLIENT_SECRET is not set"),
            "script": req.source,
            "language": LANGUAGES[req.language_id as usize].language_code.to_string(),
            "versionIndex": LANGUAGES[req.language_id as usize].version_index.to_string(),
            "stdin": req.input
        }))
        .send()
        .await
        .unwrap()
        .json::<CompilerApiResponse>()
        .await
        .unwrap_or(CompilerApiResponse {
            output: "".to_string(),
            statusCode: -1,
            memory: "-1".to_string(),
            cpuTime: "-1".to_string()
        });
    if res.statusCode < 0 {
        return HttpResponse::BadRequest().json(ExecuteResponse {
            output: res.output,
            status_code: res.statusCode,
            memory: res.memory,
            cpu_time: res.cpuTime,
        });
    }
    HttpResponse::Ok().json(ExecuteResponse {
        output: res.output,
        status_code: res.statusCode,
        memory: res.memory,
        cpu_time: res.cpuTime,
    })
}
