use actix_identity::Identity;
use actix_rt::Arbiter;
use actix_web::{put, web, HttpResponse, Responder};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Path;
// use std::sync::Mutex;
use std::{fs, sync::Arc};
use tokio::sync::Mutex;

use crate::languages::LANGUAGES;
extern crate yaml_rust;

#[derive(Deserialize)]
struct PutSubmissionsPidRequest {
    submission_id: i32,
}

#[derive(Deserialize, Serialize)]
struct SubmitRequest {
    problem_id: i32,
    language_id: i32,
    source: String,
    author: String,
    result: String,
}

#[derive(Default, Deserialize)]
struct ProblemLocation {
    id: i32,
    title: String,
    author: String,
    difficulty: i32,
    time_limit: String,
    memory_limit: i32,
    path: String,
    visible: i8,
}
#[derive(Serialize)]
struct Problem {
    problem_id: i32,
    title: String,
    author: String,
    difficulty: i64,
}

#[derive(Deserialize, Default)]
#[allow(non_snake_case)]
struct CompilerApiResponse {
    output: Option<String>,
    statusCode: i32,
    memory: Option<String>,
    cpuTime: Option<String>,
}

fn files<P: AsRef<Path>>(path: P) -> io::Result<Vec<String>> {
    Ok(fs::read_dir(path)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            if entry.file_type().ok()?.is_file() {
                Some(entry.file_name().to_string_lossy().into_owned())
            } else {
                None
            }
        })
        .collect())
}

fn format_output(s: String) -> String {
    let mut ret = "".to_string();
    let mut t = s.replace("\r", " ").replace("\n", " ").replace("a", "b");
    t.push(' ');
    let mut whitespace = false;
    for c in t.chars() {
        if !(whitespace && c == ' ') {
            ret.push(c)
        }
        whitespace = c == ' ';
    }
    ret
}

async fn judge(
    pool_data: web::Data<Arc<Mutex<sqlx::Pool<sqlx::MySql>>>>,
    testcase_num: i32,
    inputs: Vec<String>,
    problems_root: String,
    problem: ProblemLocation,
    req: SubmitRequest,
    submission_id: i32,
) {
    // ジャッジ
    let max_save_io_length = 1024;
    let mut whole_result = "AC".to_string();
    println!("testing {} testcases...", testcase_num);
    for input_path in inputs.iter() {
        // input ファイルが out 側にも存在するなら、実行してたしかめる
        let input_long = problems_root.clone() + &problem.path + "/in/" + input_path;
        let output_long = problems_root.clone()
            + &problem.path
            + "/out/"
            + &input_path.clone().replace(".in", ".out");
        if Path::new(&output_long).exists() {
            let mut input_file = File::open(input_long).expect("file not found");
            let mut input = String::new();
            input_file
                .read_to_string(&mut input)
                .expect("something went wrong reading the file");
            let mut expected_file = File::open(output_long).expect("file not found");
            let mut expected_raw = String::new();
            expected_file
                .read_to_string(&mut expected_raw)
                .expect("something went wrong reading the file");
            let expected = format_output(expected_raw.clone());

            let mut result = "AC".to_string();
            let mut will_continue = true;

            let client = reqwest::Client::new();
            println!("request: {}", &json!({
                "clientId": std::env::var("COMPILER_API_CLIENT_ID").expect("COMPILER_API_CLIENT_ID is not set"),
                "clientSecret": std::env::var("COMPILER_API_CLIENT_SECRET").expect("COMPILER_API_CLIENT_SECRET is not set"),
                "script": req.source,
                "language": LANGUAGES[req.language_id as usize].language_code.to_string(),
                "versionIndex": LANGUAGES[req.language_id as usize].version_index.to_string(),
                "stdin": input
            }).to_string());
            let res_or_err = client
                .post("https://api.jdoodle.com/v1/execute")
                .json(&json!({
                    "clientId": std::env::var("COMPILER_API_CLIENT_ID").expect("COMPILER_API_CLIENT_ID is not set"),
                    "clientSecret": std::env::var("COMPILER_API_CLIENT_SECRET").expect("COMPILER_API_CLIENT_SECRET is not set"),
                    "script": req.source,
                    "language": LANGUAGES[req.language_id as usize].language_code.to_string(),
                    "versionIndex": LANGUAGES[req.language_id as usize].version_index.to_string(),
                    "stdin": input
                }))
                .send()
                .await
                .unwrap()
                .json::<CompilerApiResponse>()
                .await;

            let res = match res_or_err {
                Ok(res) => res,
                Err(err) => CompilerApiResponse {
                    output: Some("".to_string()),
                    statusCode: err
                        .status()
                        .unwrap_or(StatusCode::from_u16(400).unwrap())
                        .to_string()
                        .parse::<i32>()
                        .unwrap_or(400),
                    memory: Some("-1".to_string()),
                    cpuTime: Some("-1".to_string()),
                },
            };

            let mut output_raw = res.output.unwrap_or("".to_string());
            let output = format_output(output_raw.clone());
            let cpu_time = res.cpuTime.unwrap_or("-1".to_string());
            let memory = res
                .memory
                .unwrap_or("1024".to_string())
                .parse::<i32>()
                .unwrap_or(1024);

            if res.statusCode == 429 {
                result = "KK".to_string();
                whole_result = "KK".to_string();
                will_continue = false;
            } else if res.statusCode == 200 {
                let cpu_time_f = cpu_time.parse::<f64>().unwrap();
                let time_limit = problem.time_limit.parse::<f64>().unwrap_or(2.0);
                let memory_limit = problem.memory_limit * 1000;
                if output_raw.starts_with("\n\n\n JDoodle - Timeout") || time_limit < cpu_time_f {
                    result = "TLE".to_string();
                    whole_result = "TLE".to_string();
                    output_raw = "(TLE)".to_string();
                    will_continue = false;
                } else if output_raw.ends_with("JDoodle - output Limit reached.\n") {
                    result = "OLE".to_string();
                    whole_result = "OLE".to_string();
                    output_raw = "(OLE)".to_string();
                    will_continue = false;
                } else if output_raw.starts_with("OCI runtime exec failed") {
                    result = "UE 200".to_string();
                    whole_result = "UE 200".to_string();
                    output_raw = "error in judge system:\n".to_string() + &output_raw;
                    will_continue = false;
                } else if cpu_time == "-1" {
                    result = "CE".to_string();
                    whole_result = "CE".to_string();
                    will_continue = false;
                } else if memory_limit < memory {
                    result = "MLE".to_string();
                    whole_result = "MLE".to_string();
                    output_raw = "(MLE)".to_string();
                    will_continue = false;
                } else if output_raw.find("Command terminated by signal").is_some() {
                    result = "RE".to_string();
                    whole_result = "RE".to_string();
                    output_raw = "RE:\n".to_string() + &output_raw;
                    will_continue = false;
                } else if output != expected {
                    result = "WA".to_string();
                    whole_result = "WA".to_string();
                    will_continue = false;
                }
            } else {
                result = format!("UE {}", res.statusCode);
                whole_result = format!("UE {}", res.statusCode);
                will_continue = false;
            }

            println!("{}", result);

            let input_reduced = if input.len() <= max_save_io_length {
                input
            } else {
                input[..max_save_io_length].to_string() + "..."
            };
            let output_reduced = if output_raw.len() <= max_save_io_length {
                output_raw
            } else {
                output_raw[..max_save_io_length].to_string() + "..."
            };
            let expected_reduced = if expected_raw.len() <= max_save_io_length {
                expected_raw
            } else {
                expected_raw[..max_save_io_length].to_string() + "..."
            };

            let pool = pool_data.lock().await;
            sqlx::query!(
                "INSERT INTO tasks (submission_id, input, output, expected, result, memory, cpu_time) VALUES (?, ?, ?, ?, ?, ?, ?);",
                submission_id,
                input_reduced,
                output_reduced,
                expected_reduced,
                result,
                memory,
                cpu_time,
            )
            .execute(&*pool)
            .await
            .unwrap();

            if !will_continue {
                break;
            }
            println!("ok");
        }
    }

    let pool = pool_data.lock().await;
    sqlx::query!(
        "UPDATE submissions SET result=? WHERE id=?;",
        whole_result,
        submission_id
    )
    .execute(&*pool)
    .await
    .unwrap();

    sqlx::query!(
        "UPDATE submissions SET result=? WHERE id=?;",
        whole_result,
        submission_id
    )
    .execute(&*pool)
    .await
    .unwrap();
}

#[put("/submissions/{submission_id}")]
async fn put_submissions_sid_handler(
    id: Identity,
    path: web::Path<PutSubmissionsPidRequest>,
    pool_data: web::Data<Arc<Mutex<sqlx::Pool<sqlx::MySql>>>>,
    arbiter_data: web::Data<Arc<Mutex<Arbiter>>>,
) -> impl Responder {
    // ログインしていなかったら弾く
    let username = id.identity().unwrap_or("".to_owned());
    if username == "" {
        return HttpResponse::Forbidden().body("not logged in".to_owned());
    }
    // let username = "tqk";

    let req: SubmitRequest;

    {
        let pool = pool_data.lock().await;
        req = sqlx::query_as!(
            SubmitRequest,
            "SELECT problem_id, language_id, source, author, result FROM submissions WHERE id=?;",
            path.submission_id
        )
        .fetch_one(&*pool)
        .await
        .unwrap_or(SubmitRequest {
            problem_id: -1,
            language_id: -1,
            source: "".to_string(),
            author: "".to_string(),
            result: "".to_string(),
        });
        if req.problem_id == -1 {
            return HttpResponse::BadRequest().body("no submission for that id");
        }
        if req.result == "WJ" {
            return HttpResponse::BadRequest().body("judging");
        }
        if req.author != username {
            return HttpResponse::Forbidden()
                .body("only submittion author or admin can trigger rejudge");
        }
        sqlx::query!(
            "UPDATE submissions SET result='WJ' WHERE id=?;",
            path.submission_id
        )
        .execute(&*pool)
        .await
        .unwrap();

        sqlx::query!(
            "DELETE FROM tasks WHERE submission_id=?;",
            path.submission_id
        )
        .execute(&*pool)
        .await
        .unwrap();
    }

    let arbiter = arbiter_data.lock().await;
    let problem;
    {
        let pool = pool_data.lock().await;
        problem = sqlx::query_as!(
            ProblemLocation,
            "SELECT * FROM problems WHERE id=?;",
            req.problem_id
        )
        .fetch_one(&*pool)
        .await
        .unwrap_or(Default::default());
    }

    if problem.visible == 0 {
        return HttpResponse::Forbidden().body("problem hidden");
    }

    // println!("submit 1")
    // 問題の情報 info を取得
    let problems_root = std::env::var("PROBLEMS_ROOT")
        .expect("PROBLEMS_ROOT not set")
        .replace("\r", "");

    // println!("submit 2")
    // テストケースの個数やパスを取得
    let mut inputs = files(problems_root.clone() + &problem.path + "/in").unwrap();
    inputs.sort();
    let mut testcase_num = 0;
    for input_path in inputs.iter() {
        let output_long = problems_root.clone()
            + &problem.path
            + "/out/"
            + &input_path.clone().replace(".in", ".out");
        println!("{}", output_long);
        if Path::new(&output_long).exists() {
            testcase_num += 1;
        }
    }

    // println!("submit 3")

    arbiter.spawn(judge(
        pool_data.clone(),
        testcase_num,
        inputs,
        problems_root,
        problem,
        req,
        path.submission_id as i32,
    ));

    // println!("submit 5")
    HttpResponse::NoContent().finish()
}
