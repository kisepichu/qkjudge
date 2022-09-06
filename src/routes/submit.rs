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

#[derive(Serialize)]
struct GetProblemsResponse {
    problems: Vec<Problem>,
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
    id: Identity,
    req: web::Json<SubmitRequest>,
    pool_data: web::Data<Arc<Mutex<sqlx::Pool<sqlx::MySql>>>>,
) -> impl Responder {
    let pool = pool_data.lock().unwrap();
    let problem_path = sqlx::query_as!(
        ProblemLocation,
        "SELECT * FROM problems WHERE id=?",
        req.problem_id
    )
    .fetch_one(&*pool)
    .await
    .unwrap_or(Default::default())
    .path;

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

    let inputs = files(problems_root.clone() + &problem_path + "/in").unwrap();
    let mut testcase_num = 0;
    let mut progress_num = 0;
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
    println!("testing {} testcases...", testcase_num);
    for input_path in inputs.iter() {
        // input ファイルが out 側にも存在するなら、実行してたしかめる
        let input_long = problems_root.clone() + &problem_path + "/in/" + input_path;
        let output_long = problems_root.clone()
            + &problem_path
            + "/out/"
            + &input_path.clone().replace(".in", ".out");
        if Path::new(&output_long).exists() {
            progress_num += 1;
            let mut input_file = File::open(input_long).expect("file not found");
            let mut input = String::new();
            input_file
                .read_to_string(&mut input)
                .expect("something went wrong reading the file");
            let mut output_file = File::open(output_long).expect("file not found");
            let mut output_raw = String::new();
            output_file
                .read_to_string(&mut output_raw)
                .expect("something went wrong reading the file");
            let output = format_output(output_raw);

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
                .unwrap();

            let expected = format_output(res.output);

            let mut result = format!("{}/{}", progress_num, testcase_num);
            let mut ok = true;

            let cpu_time = res.cpuTime.parse::<f64>().unwrap();
            let timelimit = info["timelimit"].clone().as_f64().unwrap_or(2.0);
            if timelimit < cpu_time {
                result = "TLE".to_string();
                ok = false;
            } else if expected != output {
                result = "WA".to_string();
                ok = false;
            }
            println!("{}", result);

            // db operation...

            if !ok {
                break;
            }
            println!("ok");
            break; // kari
        }
    }
    HttpResponse::NoContent().finish()
}
