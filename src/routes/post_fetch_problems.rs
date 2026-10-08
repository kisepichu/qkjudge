use std::{fs::File, io::Read, sync::Arc};

use actix_identity::Identity;
use actix_web::{
    post,
    web::{self, Bytes},
    HttpRequest, HttpResponse, Responder,
};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use sqlx::{query, query_as};
use std::collections::HashMap;
use tokio::sync::Mutex;
use yaml_rust2::{Yaml, YamlLoader};

struct ProblemSummary {
    id: i32,
    path: String,
    visible: i8,
}

fn yaml(path: String) -> Result<Yaml, HttpResponse> {
    println!("{:?}", path);
    let mut info_file = match File::open(path.clone()) {
        Ok(f) => f,
        Err(_e) => {
            println!("file {} not found", path);
            return Err(HttpResponse::InternalServerError().body("yaml: 0"));
        }
    };
    let mut info_raw = String::new();
    if info_file.read_to_string(&mut info_raw).is_err() {
        return Err(HttpResponse::InternalServerError().body("cannot read YAML"));
    }
    let docs = YamlLoader::load_from_str(&info_raw)
        .map_err(|_| HttpResponse::InternalServerError().body("invalid YAML"))?;
    docs.into_iter()
        .next()
        .ok_or_else(|| HttpResponse::InternalServerError().body("empty YAML"))
}

type HmacSha256 = Hmac<Sha256>;

pub fn validate(secret: &[u8], signature: &[u8], message: &[u8]) -> bool {
    let mut digest = [0u8; 32];
    if hex::decode_to_slice(signature, &mut digest).is_err() {
        return false;
    }
    let mut hmac = HmacSha256::new_from_slice(secret).expect("HMAC can take key of any size");
    hmac.update(message);
    hmac.verify_slice(&digest).is_ok()
}

#[post("/fetch/problems")]
pub async fn post_fetch_problems_handler(
    id: Identity,
    pool_data: web::Data<Arc<Mutex<sqlx::Pool<sqlx::MySql>>>>,
    req: HttpRequest,
    bytes: Bytes,
) -> impl Responder {
    let username = id.identity().unwrap_or("".to_owned());

    if username != "tqk" || !crate::security::trusted_origin(&req) {
        let signature = match req
            .headers()
            .get("X-Hub-Signature-256")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("sha256="))
        {
            Some(value)
                if value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()) =>
            {
                value
            }
            _ => return HttpResponse::Forbidden().body("invalid signature"),
        };
        let secret = match std::env::var("GITHUB_WEBHOOK_TOKEN") {
            Ok(secret) if !secret.is_empty() => secret,
            _ => return HttpResponse::InternalServerError().body("webhook not configured"),
        };
        if !validate(secret.as_bytes(), signature.as_bytes(), &bytes) {
            return HttpResponse::Forbidden().body("verify failed");
        }
    }

    let status = std::process::Command::new("git")
        .args(&[
            "-C",
            &std::env::var("PROBLEMS_REPO_ROOT").unwrap_or("problems".to_string()),
            "pull",
            "--rebase",
        ])
        .status()
        .expect("failed to execute git pull");

    if !status.success() {
        return HttpResponse::InternalServerError().body("git pull failed");
    }

    let root = std::env::var("PROBLEMS_ROOT")
        .expect("PROBLEMS_ROOT not set")
        .replace("\r", "");
    let config_path = match std::path::Path::new(&root)
        .canonicalize()
        .and_then(|directory| crate::security::problem_file(&directory, "qkjudge.yaml"))
    {
        Ok(path) => path.to_string_lossy().into_owned(),
        Err(_) => return HttpResponse::InternalServerError().body("invalid catalog path"),
    };

    let info = match yaml(config_path) {
        Ok(p) => p,
        Err(e) => {
            return e;
        }
    };

    if info["problems"].as_vec().is_none() {
        println!("config file error: 0");
        return HttpResponse::InternalServerError().body("post_fetch_problems_handler: 1");
    }
    let problem_paths = info["problems"].as_vec().unwrap();

    let problems: Vec<ProblemSummary>;
    let mut m: HashMap<String, bool>;
    {
        let pool = pool_data.lock().await;
        problems = query_as!(ProblemSummary, "SELECT id, path, visible FROM problems;")
            .fetch_all(&*pool)
            .await
            .unwrap();

        m = HashMap::<String, bool>::new();
        for problem in problems {
            m.insert(problem.path, false);
        }
    }

    for problem_path_yaml in problem_paths {
        if problem_path_yaml.as_str().is_none() {
            println!("config file error: 1");
            return HttpResponse::InternalServerError().body("post_fetch_problems_handler: 2");
        }
        let problem_path = problem_path_yaml.as_str().unwrap();
        let root = std::env::var("PROBLEMS_ROOT")
            .expect("PROBLEMS_ROOT not set")
            .replace("\r", "");
        let problem_path_long = match crate::security::problem_directory(&root, problem_path)
            .and_then(|directory| crate::security::problem_file(&directory, "problem.yaml"))
        {
            Ok(path) => path.to_string_lossy().into_owned(),
            Err(_) => return HttpResponse::BadRequest().body("invalid problem path"),
        };
        let probleminfo = match yaml(problem_path_long.clone()) {
            Ok(p) => p,
            Err(e) => {
                return e;
            }
        };
        if probleminfo["title"].as_str().is_none() {
            println!("title is not defined correctly in {}", problem_path_long);
            return HttpResponse::InternalServerError().body("post_fetch_problems_handler: 3");
        }
        let problem_title = probleminfo["title"].as_str().unwrap();

        if probleminfo["author"].as_str().is_none() {
            println!("author is not defined correctly in {}", problem_path_long);
            return HttpResponse::InternalServerError().body("post_fetch_problems_handler: 3");
        }
        let problem_author = probleminfo["author"].as_str().unwrap();

        if probleminfo["difficulty"].as_i64().is_none() {
            println!(
                "difficulty is not defined correctly in {}",
                problem_path_long
            );
            return HttpResponse::InternalServerError().body("post_fetch_problems_handler: 3");
        }
        let problem_difficulty = probleminfo["difficulty"].as_i64().unwrap();

        if probleminfo["time_limit"].as_str().is_none()
            || probleminfo["time_limit"]
                .as_str()
                .unwrap()
                .parse::<f64>()
                .map(|value| !value.is_finite() || value <= 0.0)
                .unwrap_or(true)
        {
            println!(
                "time_limit is not defined correctly in {}",
                problem_path_long
            );
            return HttpResponse::InternalServerError().body("post_fetch_problems_handler: 3");
        }
        let problem_time_limit = probleminfo["time_limit"].as_str().unwrap();
        if !matches!(
            probleminfo["memory_limit"].as_i64(),
            Some(1..=2_147_483_647)
        ) {
            println!(
                "memory_limit is not defined correctly in {}",
                problem_path_long
            );
            return HttpResponse::InternalServerError().body("post_fetch_problems_handler: 3");
        }
        let problem_memory_limit = probleminfo["memory_limit"].as_i64().unwrap();

        println!("problem_path: {}", problem_path_long);
        let pool = pool_data.lock().await;
        let count = query!(
            "SELECT COUNT(*) AS value FROM problems WHERE path=? LIMIT 1;",
            problem_path
        )
        .fetch_one(&*pool)
        .await
        .unwrap()
        .value;
        if 0 < count {
            if query!(
                "UPDATE problems SET title=?, author=?, difficulty=?, time_limit=?, memory_limit=?, visible=? WHERE path=?;",
                problem_title,
                problem_author,
                problem_difficulty,
                problem_time_limit,
                problem_memory_limit,
                true,
                problem_path
            )
            .execute(&*pool)
            .await
            .is_err() {
                println!("update error");
                return HttpResponse::InternalServerError().body("post_fetch_problems_handler: 4");
            }
        } else {
            println!(
                "{}, {}, {}, {}, {}",
                problem_title,
                problem_author,
                problem_difficulty,
                problem_time_limit,
                problem_memory_limit
            );
            if query!(
                "INSERT INTO problems (title, author, difficulty, time_limit, memory_limit, path, visible) values (?, ?, ?, ?, ?, ?, ?);",
                problem_title,
                problem_author,
                problem_difficulty,
                problem_time_limit,
                problem_memory_limit,
                problem_path,
                true
            )
            .execute(&*pool)
            .await
            .is_err(){
                println!("insert error");
                return HttpResponse::InternalServerError().body("post_fetch_problems_handler: 5");
            }
        }
        m.insert(problem_path.to_string(), true);
    }

    {
        let pool = pool_data.lock().await;
        for (hidden_path, visible) in &m {
            println!("{}, {}", hidden_path, visible);
            if !visible {
                if query!(
                    "UPDATE problems SET visible=? WHERE path=?;",
                    false,
                    hidden_path
                )
                .execute(&*pool)
                .await
                .is_err()
                {
                    println!("hide error");
                    return HttpResponse::InternalServerError()
                        .body("post_fetch_problems_handler: 6");
                }
            }
        }
    }

    HttpResponse::NoContent().finish()
}

#[cfg(test)]
mod tests {
    use super::validate;

    #[test]
    fn sha256_verification_authenticates_the_exact_body() {
        let signature = b"757107ea0eb2509fc211221cce984b8a37570b6d7586c22c46f4379c8b043e17";
        assert!(validate(
            b"It's a Secret to Everybody",
            signature,
            b"Hello, World!"
        ));
        assert!(!validate(
            b"It's a Secret to Everybody",
            signature,
            b"Hello, World?"
        ));
        assert!(!validate(b"wrong secret", signature, b"Hello, World!"));
        assert!(!validate(b"secret", b"x", b"body"));
        assert!(!validate(b"secret", &[0xff; 64], b"body"));
    }

    #[test]
    fn verification_accepts_signed_non_utf8_body_bytes() {
        assert!(validate(
            b"audit-secret",
            b"dd860f166d0663d5092b14f74fb52924e7a8311c43dfb2beb9bef5ebe40cab77",
            &[0xff, 0xfe]
        ));
    }
}
