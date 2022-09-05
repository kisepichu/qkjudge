



use actix_web::{
    get, HttpRequest, HttpResponse,
    Responder,
};






#[get("/")]
async fn get_index(req: HttpRequest) -> impl Responder {
    println!("Request: {req:?}");
    HttpResponse::Ok().body("Hello, World! frontend: https://tqk.blue/reactpractice/")
}
