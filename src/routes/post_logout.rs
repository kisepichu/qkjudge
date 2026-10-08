use actix_identity::Identity;
use actix_web::{post, HttpRequest, HttpResponse, Responder};

#[post("/user/logout")]
pub async fn post_logout_handler(id: Identity, request: HttpRequest) -> impl Responder {
    if !crate::security::trusted_origin(&request) {
        return HttpResponse::Forbidden().body("untrusted origin");
    }
    id.forget();
    HttpResponse::NoContent().finish()
}
