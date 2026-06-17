use actix_identity::Identity;
use actix_rt::Arbiter;
use actix_web::{post, web, HttpResponse, Responder};
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
struct SubmitRequest {
    problem_id: i32,
    language_id: i32,
    source: String,
}

#[derive(Serialize)]
struct SubmitResponse {
    id: i32,
}

// Populated from `SELECT *` via `query_as!`, so every column must have a field;
// `id`, `title`, `author`, `difficulty` are mapped but not read at runtime.
#[allow(dead_code)]
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
#[derive(Deserialize, Default)]
#[allow(non_snake_case)]
struct CompilerApiResponse {
    output: Option<String>,
    statusCode: i32,
    memory: Option<String>,
    cpuTime: Option<String>,
    // CE 判定のシグナル。JDoodle が cpuTime を返さなくなったため
    // 旧来の `cpuTime == "-1"` ヒューリスティックは AC も CE と誤判定する。
    // 代わりに `isCompiled` (CE 時 false / 実行成功時 true) を見る。
    // 欠落 (`None`) は JDoodle 仕様変化の兆候とみなし、`classify()` 内で
    // `UE 200` 早期停止して silent failure 化を防ぐ (cpuTime 欠落と同根)。
    isCompiled: Option<bool>,
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

// 判定 1 ケース分の結論。`judge()` ループ側は DB 書き込みと whole_result の更新だけ担当する。
struct Outcome {
    result: String,
    will_continue: bool,
    display_output: String,
    cpu_time: String,
    memory: i32,
}

// JDoodle の execute レスポンスから 1 テストケースの判定を返す純粋関数。
// `judge()` 内に inline していた if/else if チェーンと同等の挙動を保つ。
fn classify(
    res: CompilerApiResponse,
    expected_formatted: &str,
    problem_time_limit_str: &str,
    problem_memory_limit: i32,
) -> Outcome {
    let output_raw = res.output.unwrap_or("".to_string());
    let output_formatted = format_output(output_raw.clone());
    let cpu_time = res.cpuTime.unwrap_or("-1".to_string());
    let memory = res
        .memory
        .unwrap_or("1024".to_string())
        .parse::<i32>()
        .unwrap_or(1024);

    if res.statusCode == 429 {
        return Outcome {
            result: "KK".to_string(),
            will_continue: false,
            display_output: output_raw,
            cpu_time,
            memory,
        };
    }
    if res.statusCode != 200 {
        return Outcome {
            result: format!("UE {}", res.statusCode),
            will_continue: false,
            display_output: output_raw,
            cpu_time,
            memory,
        };
    }

    // isCompiled が欠落しているのは JDoodle 仕様変化の signal (cpuTime 欠落と同根)。
    // 「未知」を黙って AC 側に倒さず、検知可能な UE 200 で早期停止する。
    let is_compiled = match res.isCompiled {
        Some(v) => v,
        None => {
            return Outcome {
                result: "UE 200".to_string(),
                will_continue: false,
                display_output: "isCompiled missing in JDoodle response\n".to_string()
                    + &output_raw,
                cpu_time,
                memory,
            };
        }
    };

    // cpuTime 欠落時は "-1" でフォールバック済み。万一非数値文字列でも panic しないよう吸収する。
    let cpu_time_f = cpu_time.parse::<f64>().unwrap_or(-1.0);
    let time_limit = problem_time_limit_str.parse::<f64>().unwrap_or(2.0);
    let memory_limit = problem_memory_limit * 1000;

    // CE 時 JDoodle の output には "JDoodle - Timeout" 文言が中盤に混入することがあるため、
    // TLE/OLE/RE などの output ベース判定より先に isCompiled を見る。
    let (result, will_continue, display_output) = if !is_compiled {
        ("CE".to_string(), false, output_raw)
    } else if output_raw.starts_with("\n\n\n JDoodle - Timeout") || time_limit < cpu_time_f {
        ("TLE".to_string(), false, "(TLE)".to_string())
    } else if output_raw.ends_with("JDoodle - output Limit reached.\n") {
        ("OLE".to_string(), false, "(OLE)".to_string())
    } else if output_raw.starts_with("OCI runtime exec failed") {
        (
            "UE 200".to_string(),
            false,
            "error in judge system:\n".to_string() + &output_raw,
        )
    } else if memory_limit < memory {
        ("MLE".to_string(), false, "(MLE)".to_string())
    } else if output_raw.find("Command terminated by signal").is_some() {
        ("RE".to_string(), false, "RE:\n".to_string() + &output_raw)
    } else if output_formatted != expected_formatted {
        ("WA".to_string(), false, output_raw)
    } else {
        ("AC".to_string(), true, output_raw)
    };

    Outcome {
        result,
        will_continue,
        display_output,
        cpu_time,
        memory,
    }
}

async fn judge(
    pool_data: web::Data<Arc<Mutex<sqlx::Pool<sqlx::MySql>>>>,
    testcase_num: i32,
    inputs: Vec<String>,
    problems_root: String,
    problem: ProblemLocation,
    req: web::Json<SubmitRequest>,
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

            let client = reqwest::Client::new();
            // JDoodle 要求ボディには clientSecret と提出ソース / stdin が含まれる。
            // アプリログ (k8s pod log) には資格情報・提出内容を漏らさず、メタ情報のみ残す。
            println!(
                "request: language={} versionIndex={} source_len={} stdin_len={}",
                LANGUAGES[req.language_id as usize].language_code,
                LANGUAGES[req.language_id as usize].version_index,
                req.source.len(),
                input.len(),
            );
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
                    // `StatusCode::Display` は "500 Internal Server Error" のように
                    // 理由句を含む文字列で、`to_string().parse::<i32>()` だと常に
                    // 400 に潰れて実際のステータスを失う。`as_u16()` で純粋な数値に変換する。
                    statusCode: err.status().map(|s| s.as_u16() as i32).unwrap_or(400),
                    memory: Some("-1".to_string()),
                    cpuTime: Some("-1".to_string()),
                    isCompiled: Some(true),
                },
            };

            let Outcome {
                result,
                will_continue,
                display_output: output_raw,
                cpu_time,
                memory,
            } = classify(res, &expected, &problem.time_limit, problem.memory_limit);
            // 初期値 "AC" のままにしておきたい AC 以外で whole_result を更新する。
            if result != "AC" {
                whole_result = result.clone();
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

#[post("/submit")]
async fn post_submit_handler(
    id: Identity,
    req: web::Json<SubmitRequest>,
    pool_data: web::Data<Arc<Mutex<sqlx::Pool<sqlx::MySql>>>>,
    arbiter_data: web::Data<Arc<Mutex<Arbiter>>>,
) -> impl Responder {
    // ログインしていなかったら弾く
    let username = id.identity().unwrap_or("".to_owned());
    if username.is_empty() {
        return HttpResponse::Forbidden().body("not logged in".to_owned());
    }
    // let username = "tqk";

    if req.source.is_empty() {
        return HttpResponse::BadRequest().json(SubmitResponse { id: -1 });
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
    // submission を db に insert
    let submission_id: i32;
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
        problems_root,
        problem,
        req,
        submission_id,
    ));

    // println!("submit 5")
    HttpResponse::Ok().json(SubmitResponse { id: submission_id })
}

#[cfg(test)]
mod test {
    use super::{classify, format_output, CompilerApiResponse};

    fn parse_fixture(json: &str) -> CompilerApiResponse {
        serde_json::from_str(json).expect("fixture JSON should deserialize")
    }

    #[test]
    fn captured_success_response_is_ac() {
        // 2026-06-17 キャプチャ: 正しい C++ A+B (stdin "6 7") の実レスポンス。
        let res = parse_fixture(include_str!(
            "../../migration/jdoodle-response-success.json"
        ));
        let expected = format_output("13\n".to_string());
        let outcome = classify(res, &expected, "2.0", 1024);
        assert_eq!(outcome.result, "AC");
        assert!(outcome.will_continue);
    }

    #[test]
    fn captured_ce_response_is_ce_even_with_timeout_text_in_output() {
        // CE のレスポンス output には `\n\n\n JDoodle - Timeout` が中盤に混入するが、
        // isCompiled=false の判定が TLE より前に走るため CE と判定されること (= 判定順序の保証)。
        let res = parse_fixture(include_str!("../../migration/jdoodle-response-ce.json"));
        let expected = format_output("13\n".to_string());
        let outcome = classify(res, &expected, "2.0", 1024);
        assert_eq!(outcome.result, "CE");
        assert!(!outcome.will_continue);
    }

    #[test]
    fn non_numeric_cpu_time_does_not_panic_and_falls_back_to_neg_one() {
        // JDoodle 仕様変化のリグレッション保険。`unwrap_or(-1.0)` で吸収される。
        let res = CompilerApiResponse {
            output: Some("13".to_string()),
            statusCode: 200,
            memory: Some("3200".to_string()),
            cpuTime: Some("not-a-number".to_string()),
            isCompiled: Some(true),
        };
        let expected = format_output("13".to_string());
        let outcome = classify(res, &expected, "2.0", 1024);
        // -1.0 にフォールバック → time_limit (2.0) < -1.0 は false で TLE 回避 → AC。
        assert_eq!(outcome.result, "AC");
    }

    #[test]
    fn cpu_time_exceeding_time_limit_is_tle() {
        let res = CompilerApiResponse {
            output: Some("anything".to_string()),
            statusCode: 200,
            memory: Some("3200".to_string()),
            cpuTime: Some("5.0".to_string()),
            isCompiled: Some(true),
        };
        let expected = format_output("13".to_string());
        let outcome = classify(res, &expected, "2.0", 1024);
        assert_eq!(outcome.result, "TLE");
    }

    #[test]
    fn missing_is_compiled_returns_ue_200_for_safety() {
        // Copilot 指摘 (PR #27): JDoodle が将来 isCompiled も返さなくなるような仕様変化を
        // silent fail させないため、欠落時は AC 側に倒さず UE 200 で早期停止する。
        let res = CompilerApiResponse {
            output: Some("13".to_string()),
            statusCode: 200,
            memory: Some("3200".to_string()),
            cpuTime: Some("0.01".to_string()),
            isCompiled: None,
        };
        let expected = format_output("13".to_string());
        let outcome = classify(res, &expected, "2.0", 1024);
        assert_eq!(outcome.result, "UE 200");
        assert!(!outcome.will_continue);
        assert!(outcome.display_output.starts_with("isCompiled missing"));
    }

    #[test]
    fn output_mismatch_is_wa() {
        let res = CompilerApiResponse {
            output: Some("42".to_string()),
            statusCode: 200,
            memory: Some("3200".to_string()),
            cpuTime: Some("0.01".to_string()),
            isCompiled: Some(true),
        };
        let expected = format_output("13".to_string());
        let outcome = classify(res, &expected, "2.0", 1024);
        assert_eq!(outcome.result, "WA");
    }
}
