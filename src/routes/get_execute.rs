use actix_identity::Identity;
use actix_web::{get, HttpResponse, Responder};

use serde_json::json;

#[get("/execute")]
async fn get_execute_handler(id: Identity) -> impl Responder {
    let username = id.identity().unwrap_or("".to_owned());
    if username.is_empty() {
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
