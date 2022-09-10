use actix_identity::Identity;
use actix_web::{get, HttpRequest, HttpResponse, Responder};

#[get("/")]
async fn get_index_handler(req: HttpRequest, id: Identity) -> impl Responder {
    println!("Request: {req:?}");
    HttpResponse::Ok().body(format!(
        "Hello, {}!",
        id.identity().unwrap_or("guest".to_owned())
    ))
}
