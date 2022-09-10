use actix_web::{options, HttpResponse, Responder};

#[options("/")]
async fn options_0_handler() -> impl Responder {
    HttpResponse::Ok().finish()
}

#[options("{_0}")]
async fn options_1_handler() -> impl Responder {
    HttpResponse::Ok().finish()
}

#[options("{_0}/{_1}")]
async fn options_2_handler() -> impl Responder {
    HttpResponse::Ok().finish()
}
