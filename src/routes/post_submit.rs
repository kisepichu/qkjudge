use actix_identity::Identity;
use actix_rt::Arbiter;
use actix_web::{post, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::Mutex;

use crate::languages::LANGUAGES;

#[derive(Deserialize)]
struct SubmitRequest {
    problem_id: i32,
    language_id: i32,
    source: String,
}

#[derive(Serialize)]
struct SubmitResponse {
    id: i32,
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

fn format_output(s: &str) -> String {
    let mut output = String::with_capacity(s.len() + 1);
    let mut whitespace = false;
    for character in s.chars().chain(std::iter::once(' ')) {
        let character = match character {
            '\r' | '\n' => ' ',
            character => character,
        };
        if !(whitespace && character == ' ') {
            output.push(character);
        }
        whitespace = character == ' ';
    }
    output
}

async fn judge(
    pool_data: web::Data<Arc<Mutex<sqlx::Pool<sqlx::MySql>>>>,
    testcase_num: i32,
    inputs: Vec<String>,
    directory: PathBuf,
    problem: ProblemLocation,
    req: web::Json<SubmitRequest>,
    submission_id: i32,
) {
    // ジャッジ
    let max_save_io_length = 1024;
    let mut whole_result = "AC".to_string();
    println!("testing {} testcases...", testcase_num);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap();
    for input_path in inputs.iter() {
        // input ファイルが out 側にも存在するなら、実行してたしかめる
        let input = crate::security::problem_file(&directory, Path::new("in").join(input_path))
            .and_then(std::fs::read_to_string);
        let expected = crate::security::problem_file(
            &directory,
            Path::new("out").join(input_path.replace(".in", ".out")),
        )
        .and_then(std::fs::read_to_string);
        let (input, expected_raw) = match (input, expected) {
            (Ok(input), Ok(expected)) => (input, expected),
            _ => {
                whole_result = "UE 500".to_owned();
                break;
            }
        };
        let expected = format_output(&expected_raw);

        let mut result = "AC".to_string();
        let mut will_continue = true;

        let response = client
                .post("https://api.jdoodle.com/v1/execute")
                .json(&json!({
                    "clientId": std::env::var("COMPILER_API_CLIENT_ID").expect("COMPILER_API_CLIENT_ID is not set"),
                    "clientSecret": std::env::var("COMPILER_API_CLIENT_SECRET").expect("COMPILER_API_CLIENT_SECRET is not set"),
                    "script": req.source,
                    "language": LANGUAGES[req.language_id as usize].language_code,
                    "versionIndex": LANGUAGES[req.language_id as usize].version_index,
                    "stdin": input
                }))
                .send()
                .await;
        let res_or_err = match response {
            Ok(response) => response.json::<CompilerApiResponse>().await,
            Err(error) => Err(error),
        };

        let res = match res_or_err {
            Ok(res) => res,
            Err(err) => CompilerApiResponse {
                output: Some("".to_string()),
                statusCode: i32::from(err.status().map(|status| status.as_u16()).unwrap_or(502)),
                memory: Some("-1".to_string()),
                cpuTime: Some("-1".to_string()),
            },
        };

        let mut output_raw = res.output.unwrap_or("".to_string());
        let output = format_output(&output_raw);
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
            let cpu_time_f = match cpu_time.parse::<f64>() {
                Ok(value) if value.is_finite() => value,
                _ => {
                    whole_result = "UE 502".to_owned();
                    break;
                }
            };
            let time_limit = problem.time_limit.parse::<f64>().unwrap_or(2.0);
            let memory_limit = i64::from(problem.memory_limit) * 1000;
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
            } else if memory_limit < i64::from(memory) {
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

        let input_reduced = crate::security::truncate_io(input, max_save_io_length);
        let output_reduced = crate::security::truncate_io(output_raw, max_save_io_length);
        let expected_reduced = crate::security::truncate_io(expected_raw, max_save_io_length);

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

#[post("/submit")]
pub async fn post_submit_handler(
    id: Identity,
    req: web::Json<SubmitRequest>,
    pool_data: web::Data<Arc<Mutex<sqlx::Pool<sqlx::MySql>>>>,
    arbiter_data: web::Data<Arc<Mutex<Arbiter>>>,
) -> impl Responder {
    // ログインしていなかったら弾く
    let username = id.identity().unwrap_or("".to_owned());
    if username == "" {
        return HttpResponse::Forbidden().body("not logged in".to_owned());
    }

    if req.source == "" {
        return HttpResponse::BadRequest().json(SubmitResponse { id: -1 });
    }
    if usize::try_from(req.language_id)
        .ok()
        .and_then(|index| LANGUAGES.get(index))
        .is_none()
    {
        return HttpResponse::BadRequest().body("invalid language_id");
    }
    if req.source.len() > 65_535 {
        return HttpResponse::BadRequest().body("source is too long");
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

    let directory = match crate::security::problem_directory(&problems_root, &problem.path) {
        Ok(directory) => directory,
        Err(_) => return HttpResponse::BadRequest().body("invalid problem path"),
    };
    let inputs = match crate::security::input_files(&directory) {
        Ok(inputs) if !inputs.is_empty() => inputs,
        _ => return HttpResponse::BadRequest().body("testcases unavailable"),
    };
    let testcase_num = inputs.len() as i32;

    // println!("submit 3")
    // submission を db に insert
    let mut submission_id: i32 = 0;
    {
        let pool = pool_data.lock().await;
        submission_id = sqlx::query!(
            "INSERT INTO submissions (date, author, problem_id, testcase_num, result, language_id, source) VALUES (NOW(), ?, ?, ?, ?, ?, ?);",
            username,
            req.problem_id,
            testcase_num,
            "WJ".to_string(),
            req.language_id,
            req.source
        )
        .execute(&*pool)
        .await
        .unwrap()
        .last_insert_id() as i32;
    }
    println!("submission id: {}", submission_id);

    arbiter.spawn(judge(
        pool_data.clone(),
        testcase_num,
        inputs,
        directory,
        problem,
        req,
        submission_id as i32,
    ));

    // println!("submit 5")
    HttpResponse::Ok().json(SubmitResponse {
        id: submission_id as i32,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn answer_comparison_does_not_change_letters() {
        assert_ne!(super::format_output("a"), super::format_output("b"));
        assert_eq!(
            super::format_output("1\n2\r\n"),
            super::format_output("1 2")
        );
    }
}
