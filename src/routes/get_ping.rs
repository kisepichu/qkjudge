use actix_web::{get, HttpResponse, Responder};

#[get("/ping")]
async fn get_ping_handler() -> impl Responder {
    HttpResponse::Ok().body("pong")
}
