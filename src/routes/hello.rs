use actix_identity::Identity;
use actix_web::{get, HttpResponse, Responder};

#[get("/hello")]
async fn get_hello(id: Identity) -> impl Responder {
    HttpResponse::Ok().body(format!(
        "Hello, {}!",
        id.identity().unwrap_or("guest".to_owned())
    ))
}
