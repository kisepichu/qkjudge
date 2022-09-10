use actix_web::{options, HttpResponse, Responder};

#[options("{_}")]
async fn options_any_handler() -> impl Responder {
    HttpResponse::Ok().finish()
}
