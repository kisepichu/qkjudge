use actix_identity::Identity;
use actix_web::{delete, HttpResponse, Responder};

#[delete("/user/logout")]
pub async fn delete_logout(id: Identity) -> impl Responder {
    id.forget();
    HttpResponse::NoContent().finish()
}
