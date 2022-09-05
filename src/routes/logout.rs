use actix_identity::Identity;
use actix_web::{
    delete, HttpResponse,
    Responder,
};

#[delete("/logout")]
pub async fn delete_logout(id: Identity) -> impl Responder {
    id.forget();
    HttpResponse::Ok().finish()
}
