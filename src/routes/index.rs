use actix_cors::Cors;
use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::cookie::SameSite;
use actix_web::web::Data;
use actix_web::{
    delete, get, http, middleware, options, post, web, App, HttpRequest, HttpResponse, HttpServer,
    Responder,
};
use bcrypt::{hash, verify, DEFAULT_COST};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::*;

#[get("/")]
async fn get_index(req: HttpRequest) -> impl Responder {
    println!("Request: {req:?}");
    HttpResponse::Ok().body("Hello, World! frontend: https://tqk.blue/reactpractice/")
}
