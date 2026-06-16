use std::{fs::File, io::Read, sync::Arc};

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
use yaml_rust::{Yaml, YamlLoader};

// `id` and `visible` are not read but must exist to match the columns
// selected by the `query_as!` macro below.
#[allow(dead_code)]
struct ProblemSummary {
    id: i32,
    path: String,
    visible: i8,
}

// `HttpResponse` is the error type these handlers pass around directly;
// boxing it would only shuffle the size to the call sites without benefit.
#[allow(clippy::result_large_err)]
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
    info_file
        .read_to_string(&mut info_raw)
        .expect("something went wrong reading the file");
    let docs = YamlLoader::load_from_str(&info_raw).unwrap();
    Ok(docs[0].to_owned())
}

type HmacSha256 = Hmac<Sha256>;

// `signature` は X-Hub-Signature-256 から "sha256=" を剥がした hex 文字列のバイト列を期待する。
// 期待 hex とバイト単位の固定時間比較を行う (長さが違えば fixed_time_eq は false を返す)。
// 署名値は機密相当なのでログには出さない。
pub fn validate(secret: &[u8], signature: &[u8], message: &[u8]) -> bool {
    let mut hmac = HmacSha256::new_from_slice(secret).expect("HMAC can take key of any size");
    hmac.update(message);
    let expected = hex::encode(hmac.finalize().into_bytes());
    crypto::util::fixed_time_eq(expected.as_bytes(), signature)
}

#[post("/fetch/problems")]
async fn post_fetch_problems_handler(
    pool_data: web::Data<Arc<Mutex<sqlx::Pool<sqlx::MySql>>>>,
    req: HttpRequest,
    bytes: Bytes,
) -> impl Responder {
    // 署名検証は常に必須。GitHub Webhook 専用エンドポイントなので Identity は使わない。
    // ヘッダ欠落 / 非 ASCII / プレフィクス不一致のいずれでも 403 を返し panic させない。
    let sign_github = match req
        .headers()
        .get("X-Hub-Signature-256")
        .and_then(|s| s.to_str().ok())
        .and_then(|s| s.strip_prefix("sha256="))
    {
        Some(s) => s.as_bytes(),
        None => return HttpResponse::Forbidden().body("signature is not set in header"),
    };

    let secret = std::env::var("GITHUB_WEBHOOK_TOKEN").expect("env GITHUB_WEBHOOK_TOKEN not set");

    if !validate(secret.as_bytes(), sign_github, &bytes) {
        return HttpResponse::Forbidden().body("verify failed");
    }

    let status = std::process::Command::new("git")
        .args([
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

    let config_path = std::env::var("PROBLEMS_ROOT")
        .expect("PROBLEMS_ROOT not set")
        .replace("\r", "")
        + "/qkjudge.yaml";

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
        let problem_path_long = std::env::var("PROBLEMS_ROOT")
            .expect("PROBLEMS_ROOT not set")
            .replace("\r", "")
            + problem_path
            + "/problem.yaml";
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
                .is_err()
        {
            println!(
                "time_limit is not defined correctly in {}",
                problem_path_long
            );
            return HttpResponse::InternalServerError().body("post_fetch_problems_handler: 3");
        }
        let problem_time_limit = probleminfo["time_limit"].as_str().unwrap();
        if probleminfo["memory_limit"].as_i64().is_none() {
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
            if !visible
                && query!(
                    "UPDATE problems SET visible=? WHERE path=?;",
                    false,
                    hidden_path
                )
                .execute(&*pool)
                .await
                .is_err()
            {
                println!("hide error");
                return HttpResponse::InternalServerError().body("post_fetch_problems_handler: 6");
            }
        }
    }

    HttpResponse::NoContent().finish()
}

#[cfg(test)]
mod test {
    use super::validate;
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    fn sign_hex(secret: &[u8], message: &[u8]) -> String {
        let mut hmac = HmacSha256::new_from_slice(secret).expect("HMAC can take key of any size");
        hmac.update(message);
        hex::encode(hmac.finalize().into_bytes())
    }

    #[test]
    fn it_returns_true_when_signature_and_message_match() {
        let secret = b"some-secret";
        let message = b"blah-blah-blah";
        let sig = sign_hex(secret, message);
        assert!(validate(secret, sig.as_bytes(), message));
    }

    #[test]
    fn it_returns_false_when_signature_and_message_do_not_match() {
        let secret = b"some-secret";
        let message = b"blah-blah-blah";
        let bad_message = b"blah-blah-blah?";
        let sig = sign_hex(secret, message);
        assert!(!validate(secret, sig.as_bytes(), bad_message));
    }

    #[test]
    fn it_returns_false_when_signature_length_differs() {
        // fixed_time_eq は長さ違いを false で返す。truncate された署名で誤って通らないことを確認。
        let secret = b"some-secret";
        let message = b"blah-blah-blah";
        let sig = sign_hex(secret, message);
        assert!(!validate(secret, &sig.as_bytes()[..sig.len() - 1], message));
    }
}
