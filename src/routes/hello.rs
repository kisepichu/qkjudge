use actix_cors::Cors;
use actix_identity::{CookieIdentityPolicy, Identity, IdentityService};
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

#[get("/hello")]
async fn get_hello(id: Identity) -> impl Responder {
    HttpResponse::Ok().body(format!(
        "Hello, {}!",
        id.identity().unwrap_or("guest".to_owned())
    ))
}
