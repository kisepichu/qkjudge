use actix_identity::Identity;
use actix_web::{get, HttpResponse, Responder};
use serde::Serialize;

#[derive(Serialize)]
struct WhoamiResponse {
    username: String,
}

#[get("/user/whoami")]
async fn get_whoami(id: Identity) -> impl Responder {
    let username = id.identity().unwrap_or("".to_owned());
    if username == "" {
        return HttpResponse::Forbidden().body("not logged in".to_owned());
    }
    HttpResponse::Ok().json(WhoamiResponse { username: username })
}
