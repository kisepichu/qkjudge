use actix_identity::Identity;
use actix_web::{get, HttpRequest, HttpResponse, Responder};

#[get("/")]
async fn get_index(req: HttpRequest, id: Identity) -> impl Responder {
    println!("Request: {req:?}");
    HttpResponse::Ok().body(format!(
        "Hello, {}!",
        id.identity().unwrap_or("guest".to_owned())
    ))
}
