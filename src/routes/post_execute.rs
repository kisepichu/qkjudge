use actix_identity::Identity;
use actix_web::{post, web, HttpResponse, Responder};

use reqwest::StatusCode;
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
    memory: Option<String>,
    cpuTime: Option<String>,
}

#[derive(Serialize)]
struct ExecuteResponse {
    output: String,
    status_code: i32,
    result: String,
    memory: String,
    cpu_time: String,
}

#[post("/execute")]
async fn post_execute_handler(req: web::Json<ExecuteRequest>, _id: Identity) -> impl Responder {
    // let username = id.identity().unwrap_or("".to_owned());
    // if username == "" {
    //     return HttpResponse::Forbidden().body("not logged in".to_owned());
    // }
    if req.source == "" {
        return HttpResponse::BadRequest().json(ExecuteResponse {
            output: "".to_string(),
            status_code: 400,
            result: "".to_string(),
            memory: "-1".to_string(),
            cpu_time: "-1".to_string(),
        });
    }

    let client = reqwest::Client::new();
    let res_or_err = client
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
        .await;
    let res = match res_or_err {
        Ok(res) => res,
        Err(err) => CompilerApiResponse {
            output: "".to_string(),
            statusCode: err
                .status()
                .unwrap_or(StatusCode::from_u16(400).unwrap())
                .to_string()
                .parse::<i32>()
                .unwrap_or(400),
            memory: Some("-1".to_string()),
            cpuTime: Some("-1".to_string()),
        },
    };
    let cpu_time = res.cpuTime.unwrap_or("-1".to_string());
    let memory = res.memory.unwrap_or("-1".to_string());
    let mut result = "OK".to_string();
    let mut output = res.output.clone();

    if res.statusCode == 429 {
        result = "KK".to_string();
    } else if res.statusCode == 200 {
        if res.output.starts_with("\n\n\n JDoodle - Timeout") {
            result = "TLE".to_string();
            output = "(TLE)".to_string()
        } else if res.output.ends_with("JDoodle - output Limit reached.\n") {
            result = "OLE".to_string();
            output = "(OLE)".to_string();
        } else if cpu_time == "-1" {
            result = "CE".to_string();
        } else if res.output.find("Command terminated by signal").is_some() {
            result = "RE".to_string();
            output = "RE:\n".to_string() + &res.output;
        } else {
            // result = "OK".to_string();
        }
    } else {
        result = format!("UE {}", res.statusCode);
    }
    HttpResponse::Ok().json(ExecuteResponse {
        output: output,
        status_code: res.statusCode,
        result: result,
        memory: memory,
        cpu_time: cpu_time,
    })
}
