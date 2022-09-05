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

#[get("/whoami")]
async fn get_whoami(id: Identity) -> impl Responder {
    let username = id.identity().unwrap_or("".to_owned());
    if username == "" {
        return HttpResponse::Forbidden().body("not logged in".to_owned());
    }
    HttpResponse::Ok().body(username)
}
