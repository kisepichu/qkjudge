use actix_identity::Identity;
use actix_web::{post, HttpResponse, Responder};

#[post("/user/logout")]
async fn post_logout_handler(id: Identity) -> impl Responder {
    id.forget();
    HttpResponse::NoContent().finish()
}
