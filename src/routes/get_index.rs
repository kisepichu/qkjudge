use actix_identity::Identity;
use actix_web::{get, HttpResponse, Responder};

#[get("/")]
pub async fn get_index_handler(id: Identity) -> impl Responder {
    HttpResponse::Ok().body(format!(
        "Hello, {}!",
        id.identity().unwrap_or("guest".to_owned())
    ))
}
