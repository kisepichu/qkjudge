use actix_web::{options, HttpResponse, Responder};

#[options("{_}")]
async fn options_any() -> impl Responder {
    HttpResponse::Ok().finish()
}
