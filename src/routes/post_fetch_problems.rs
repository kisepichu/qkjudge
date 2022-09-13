use std::io::{BufRead, BufReader};

use actix_identity::Identity;
use actix_web::{post, HttpResponse, Responder};

#[post("/fetch/problems")]
async fn post_fetch_problems_handler(id: Identity) -> impl Responder {
    let username = id.identity().unwrap_or("".to_owned());
    if username == "" {
        return HttpResponse::Forbidden().body("not logged in".to_owned());
    } else if username != "admin" {
        return HttpResponse::Forbidden().body("not permitted".to_owned());
    }
    let output = match std::process::Command::new("git")
        .args(&[
            "-C",
            &std::env::var("PROBLEMS_REPO_ROOT").unwrap_or("problems".to_string()),
            "pull",
        ])
        .output()
    {
        Ok(c) => c,
        Err(_e) => return HttpResponse::InternalServerError().body("failed to start pull"),
    };
    println!("{:?}", String::from_utf8_lossy(&output.stdout));

    HttpResponse::NoContent().finish()
}
