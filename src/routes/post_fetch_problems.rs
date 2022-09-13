use std::{fs::File, io::Read, sync::Arc};

use actix_identity::Identity;
use actix_web::{post, web, HttpResponse, Responder};
use sqlx::{query, query_as};
use std::collections::HashMap;
use tokio::sync::Mutex;
use yaml_rust::{Yaml, YamlLoader};

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
    info_file
        .read_to_string(&mut info_raw)
        .expect("something went wrong reading the file");
    let docs = YamlLoader::load_from_str(&info_raw).unwrap();
    Ok(docs[0].to_owned())
}

#[post("/fetch/problems")]
async fn post_fetch_problems_handler(
    id: Identity,
    pool_data: web::Data<Arc<Mutex<sqlx::Pool<sqlx::MySql>>>>,
) -> impl Responder {
    // let username = id.identity().unwrap_or("".to_owned());
    // if username == "" {
    //     return HttpResponse::Forbidden().body("not logged in".to_owned());
    // } else if username != "admin" {
    //     return HttpResponse::Forbidden().body("not permitted".to_owned());
    // }

    let output = match std::process::Command::new("git")
        .args(&[
            "-C",
            &std::env::var("PROBLEMS_REPO_ROOT").unwrap_or("problems".to_string()),
            "pull",
        ])
        .output()
    {
        Ok(c) => c,
        Err(_e) => {
            println!("failed to start pull");
            return HttpResponse::InternalServerError().body("post_fetch_problems_handler: 0");
        }
    };
    println!("{:?}", String::from_utf8_lossy(&output.stdout));

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
            + problem_path.clone()
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
            "SELECT COUNT(*) AS value FROM problems WHERE path=?;",
            problem_path.clone()
        )
        .fetch_one(&*pool)
        .await
        .unwrap()
        .value;
        println!("count: {}", count);
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
