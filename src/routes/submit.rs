use actix_identity::Identity;
use actix_web::{post, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Path;
use std::sync::*;
use yaml_rust::YamlLoader;
extern crate yaml_rust;

#[derive(Deserialize)]
struct SubmitRequest {
    problem_id: i32,
    language: String,
    source: String,
}

#[derive(Default, Deserialize)]
struct ProblemLocation {
    id: i32,
    path: String,
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
    output: String,
    statusCode: i32,
    memory: String,
    cpuTime: String,
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

#[post("/submit")]
async fn post_submit(
    _id: Identity,
    req: web::Json<SubmitRequest>,
    pool_data: web::Data<Arc<Mutex<sqlx::Pool<sqlx::MySql>>>>,
) -> impl Responder {
    let max_save_io_length = 1024;
    // ログインしていなかったら弾く
    // let username = id.identity().unwrap_or("".to_owned());
    // if username == "" {
    //     return HttpResponse::Forbidden().body("not logged in".to_owned());
    // }
    let username = "tqk";
    // 問題のフォルダ problem_path を取得
    let mut pool = pool_data.lock().unwrap();
    let problem_path = sqlx::query_as!(
        ProblemLocation,
        "SELECT * FROM problems WHERE id=?;",
        req.problem_id
    )
    .fetch_one(&*pool)
    .await
    .unwrap_or(Default::default())
    .path;

    // 問題の情報 info を取得
    let problems_root = std::env::var("PROBLEMS_ROOT")
        .expect("PROBLEMS_ROOT not set")
        .replace("\r", "");
    let info_path = problems_root.clone() + &problem_path + "/problem.yaml";
    println!("{:?}", info_path);
    let mut info_file = File::open(info_path).expect("file not found");
    let mut info_raw = String::new();
    info_file
        .read_to_string(&mut info_raw)
        .expect("something went wrong reading the file");
    let docs = YamlLoader::load_from_str(&info_raw).unwrap();
    let info = &docs[0];

    // テストケースの個数やパスを取得
    let inputs = files(problems_root.clone() + &problem_path + "/in").unwrap();
    let mut testcase_num = 0;
    for input_path in inputs.iter() {
        let output_long = problems_root.clone()
            + &problem_path
            + "/out/"
            + &input_path.clone().replace(".in", ".out");
        println!("{}", output_long);
        if Path::new(&output_long).exists() {
            testcase_num += 1;
        }
    }

    // submission を db に insert
    let submission_id = sqlx::query!(
        "INSERT INTO submissions (date, author, problem_id, testcase_num, result, language, source) VALUES (NOW(), ?, ?, ?, ?, ?, ?);",
        username,
        req.problem_id,
        testcase_num,
        "WJ".to_string(),
        req.language,
        req.source
    )
    .execute(&*pool)
    .await
    .unwrap()
    .last_insert_id();
    std::mem::drop(pool);
    println!("submission id: {}", submission_id);

    // ジャッジ
    let mut whole_result = "AC".to_string();
    println!("testing {} testcases...", testcase_num);
    for input_path in inputs.iter() {
        // input ファイルが out 側にも存在するなら、実行してたしかめる
        let input_long = problems_root.clone() + &problem_path + "/in/" + input_path;
        let output_long = problems_root.clone()
            + &problem_path
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
            let expected = format_output(expected_raw);

            let mut result = "AC".to_string();
            let mut will_continue = true;

            let client = reqwest::Client::new();
            let res = client
                .post("https://api.jdoodle.com/v1/execute")
                .json(&json!({
                    "clientId": std::env::var("COMPILER_API_CLIENT_ID").expect("COMPILER_API_CLIENT_ID is not set"),
                    "clientSecret": std::env::var("COMPILER_API_CLIENT_SECRET").expect("COMPILER_API_CLIENT_SECRET is not set"),
                    "script": req.source,
                    "language": req.language,
                    "stdin": input
                }))
                .send()
                .await
                .unwrap()
                .json::<CompilerApiResponse>()
                .await
                .unwrap_or(Default::default());
            let output = format_output(res.output);

            if res.statusCode == 429 {
                result = "KK".to_string();
                whole_result = "KK".to_string();
                will_continue = false;
            } else if res.statusCode == 200 {
                let cpu_time = res.cpuTime.parse::<f64>().unwrap();
                let timelimit = info["timelimit"].clone().as_f64().unwrap_or(2.0);
                if timelimit < cpu_time {
                    result = "TLE".to_string();
                    whole_result = "TLE".to_string();
                    will_continue = false;
                } else if output != expected {
                    result = "WA".to_string();
                    whole_result = "WA".to_string();
                    will_continue = false;
                }
            } else {
                println!("{}", res.statusCode);
                result = "UK".to_string();
                whole_result = "UK".to_string();
                will_continue = false;
            }

            println!("{}", result);

            let input_reduced = if input.len() <= max_save_io_length {
                input
            } else {
                input[..max_save_io_length].to_string() + "..."
            };
            let output_reduced = if output.len() <= max_save_io_length {
                output
            } else {
                output[..max_save_io_length].to_string() + "..."
            };
            let expected_reduced = if expected.len() <= max_save_io_length {
                expected
            } else {
                expected[..max_save_io_length].to_string() + "..."
            };

            pool = pool_data.lock().unwrap();
            sqlx::query!(
                "INSERT INTO tasks (submission_id, input, output, expected, result, memory, cpu_time) VALUES (?, ?, ?, ?, ?, ?, ?);",
                submission_id,
                input_reduced,
                output_reduced,
                expected_reduced,
                result,
                res.memory,
                res.cpuTime,
            )
            .execute(&*pool)
            .await
            .unwrap();
            std::mem::drop(pool);

            if !will_continue {
                break;
            }
            println!("ok");
        }
    }

    pool = pool_data.lock().unwrap();
    sqlx::query!(
        "UPDATE submissions SET result=? WHERE id=?;",
        whole_result,
        submission_id
    )
    .execute(&*pool)
    .await
    .unwrap();

    HttpResponse::NoContent().finish()
}
