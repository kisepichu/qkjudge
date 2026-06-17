# TASK-007: JDoodle CE 誤判定の修正 (Phase 6 / 本命)

## 参照

- `docs/MIGRATION.md`「本来の目的」
- `src/routes/post_submit.rs` (`CompilerApiResponse`、判定ロジック、`judge()`)
- `src/routes/get_execute.rs` (`/credit-spent` を `.text()` で受けるのみ、struct パースは無いので整合修正不要)
- `src/languages.rs` (C++ = id 0 / `language_code "cpp17"` / `version_index "1"`)
- `migration/jdoodle-response-success.json` / `migration/jdoodle-response-ce.json` (2026-06-17 キャプチャ)

## 概要

JDoodle (`api.jdoodle.com/v1/execute`) が成功時に `cpuTime` を返さなくなったため、
`post_submit.rs` の `else if cpu_time == "-1" { CE }` が**正しい提出を全部 CE と誤判定**している。
現行 JDoodle レスポンスに合わせて CE 判定を直す。

## 根拠 (確定済み)

旧公開 API のタスクから:
- 古い AC: `memory="4736", cpu_time="0.00"`。
- 最近 CE (task 321 / submission 58、正しい C++ A+B): `output="13", expected="13\n", memory="3072", cpu_time="-1"`。
- → JDoodle は `output`/`memory`(string) は今も返すが **`cpuTime` を返さない**。
  (数値化なら struct パース失敗で "UE 400" になるはず → よって欠落/null。)

## 設計メモ

- **キャプチャ結果 (2026-06-17, cpp17/v1)**:
  - 成功: `{"output":"13","statusCode":200,"memory":"3200","cpuTime":null,"isCompiled":true,"isExecutionSuccess":true}`
  - CE: `{"output":"\n... error: expected ... \n\n\n JDoodle - Timeout \n...","statusCode":200,"memory":null,"cpuTime":null,"isCompiled":false,"isExecutionSuccess":false}`
  - **決定的シグナル**: `isCompiled` (CE 時 false / それ以外 true)。`cpuTime` は欠落、`statusCode` は両者とも 200 で区別不能。
  - **注意**: CE 時の `output` に `\n\n\n JDoodle - Timeout` 文字列が*中盤に*混入する。現状の TLE 判定は `starts_with` で先頭 `\n` 1 個だけ → false でたまたまセーフだが、フォーマット変化に耐えるよう CE 判定を TLE より**前**に置く。
- **修正方針** (適用済):
  - `CompilerApiResponse` に `isCompiled: Option<bool>` を追加。**`None` は JDoodle 仕様変化の signal とみなし `UE 200` で早期停止** (cpuTime 欠落と同根、PR #27 Copilot レビューで初期案 `unwrap_or(true)` から変更)。
  - 判定ロジックで CE 判定を `cpu_time == "-1"` → `!is_compiled` に差し替え、`TLE/OLE/OCI` より前に移動。
  - `cpu_time.parse::<f64>().unwrap()` を `.unwrap_or(-1.0)` に変更 (非数値文字列でも panic しない保険)。
  - err 時の `statusCode` 復元を `StatusCode::Display::to_string().parse::<i32>()` (常に 400 に潰れる) から `s.as_u16() as i32` に変更 (PR #27 Copilot レビュー対応)。
- 影響範囲: 判定の各分岐 (TLE/OLE/CE/MLE/RE/WA/AC) を、現行レスポンスで一通り検証 (staging で実機)。

## チェックリスト

- [x] 現行 JDoodle レスポンス (成功 / CE) をキャプチャし本タスクに記録 (`migration/jdoodle-response-{success,ce}.json`)
- [x] 判定ロジックを現行仕様に合わせて修正: CE 判定を `cpu_time == "-1"` → `!is_compiled` へ差し替え、TLE より前に移動
- [x] `cpu_time.parse::<f64>().unwrap()` を `unwrap_or(-1.0)` で安全化
- [x] 判定ロジックを `classify()` 純粋関数に抽出 (テスト容易化)
- [x] `classify()` のユニットテスト 6 件追加 (AC / CE / 非数値 cpuTime / TLE / WA / `isCompiled` None UE 200)
- [x] 手元 `docker compose` で実 JDoodle 呼び出しを含む AC / CE を 1 件ずつ確認 (PR #27 にて完了)
- [x] dev へ merge 後 staging で同じ検証 (submission id 3=AC / 4=CE で確認、2026-06-17)
- [x] `cargo test` (9 passed) / `cargo clippy --all-targets -- -D warnings` (warning なし)

## 完了条件

- [x] 正しい提出が AC になる (CE 誤判定が解消)
- [x] 実際のコンパイルエラーは CE と判定される
- [x] cpuTime が非数値文字列でも panic しない

## 手元検証手順 (compose, dev 未マージ前)

```sh
# 1. .env 準備 (JDoodle key を .env.k8s.staging から流用、その他は適当)
cp .env.example .env
# .env を編集して COMPILER_API_CLIENT_ID / COMPILER_API_CLIENT_SECRET を埋める
# SESSION_KEY も `openssl rand -hex 32` で埋めると再起動間でログイン維持できる

# 2. compose up (problems は entrypoint が qkjudge-problems#dist から自動 clone)
docker compose up --build

# 3. 別ターミナルから DB に問題を register (webhook 経由は HMAC 計算が必要なので直 INSERT が楽)
#    qkjudge-problems の dist にある問題から 1 件選び、path / time_limit / memory_limit を埋める。
docker compose exec mariadb mariadb -uqkjudge -p"$MARIADB_PASSWORD" qkjudge -e \
  "INSERT INTO problems (title, author, difficulty, time_limit, memory_limit, path, visible) \
   VALUES ('A+B', 'kisepichu', 0, '2.0', 256, '/aplusb', 1);"

# 4. /signup と /login で cookie 取得
curl -sS -c /tmp/qj.cookie -b /tmp/qj.cookie \
  -H 'Content-Type: application/json' \
  -d '{"username":"local","password":"pw"}' \
  http://127.0.0.1:8080/signup
curl -sS -c /tmp/qj.cookie -b /tmp/qj.cookie \
  -H 'Content-Type: application/json' \
  -d '{"username":"local","password":"pw"}' \
  http://127.0.0.1:8080/login

# 5. AC ケース (正しい C++ A+B)
curl -sS -b /tmp/qj.cookie -H 'Content-Type: application/json' \
  -d '{"problem_id":1,"language_id":0,"source":"#include <iostream>\nusing namespace std;\nint main(){int a,b;cin>>a>>b;cout<<a+b<<endl;return 0;}"}' \
  http://127.0.0.1:8080/submit
# → 数秒後に GET /submissions/{id} で result が "AC" になっていることを確認

# 6. CE ケース (わざと壊した C++)
curl -sS -b /tmp/qj.cookie -H 'Content-Type: application/json' \
  -d '{"problem_id":1,"language_id":0,"source":"#include <iostream>\nthis is not valid c++\nint main(){return 0;}"}' \
  http://127.0.0.1:8080/submit
# → result が "CE" になっていることを確認
```

JDoodle は 1 リクエスト = 1 クレジット (200/日)。手元検証で 2-4 コール、staging で 2-4 コール程度なら問題なし。

## 作業ログ

- 2026-05-31: タスク生成。根本原因を旧 API 実データで確定。現行レスポンスのキャプチャ待ち。
- 2026-06-17: 現行 JDoodle (cpp17/v1) のレスポンスをキャプチャ。CE シグナルを `isCompiled:false` に確定し
  `src/routes/post_submit.rs` を修正 (struct に `isCompiled` 追加 / 判定順を CE 最優先に変更 / `parse::<f64>` 安全化)。
- 2026-06-17 (続): 判定ロジックを `classify()` 純粋関数に抽出。fixture (`migration/jdoodle-response-{success,ce}.json`) を
  `include_str!` で読み込むユニットテストを 6 件追加 (AC / CE 判定順序保証 / 非数値 cpuTime panic 回避 / TLE / WA /
  `isCompiled` None UE 200)。`cargo test` 9 passed、`cargo clippy --all-targets -- -D warnings` clean。
  手元 `docker compose` で AC / CE 実機確認済。残りは staging 実機。
- 2026-06-17 (PR #27 Copilot レビュー対応): JDoodle err 時の `statusCode` 復元バグ修正
  (`Display::to_string().parse::<i32>()` → `as_u16() as i32`)、`isCompiled` 欠落時の
  silent failure 防止 (`UE 200` 早期停止)、debug log の secret 漏洩対策
  (`println!("request: ...")` から clientSecret / source / stdin を除去しメタ情報のみに変更)。
- 2026-06-17 (staging 実機検証): PR #27 merge 後の auto-deploy で staging に新コードが反映。
  検証中に webhook URL/secret が PR #23 host rename + TASK-004 auth harden から放置されていた
  ことが判明 (`api.{dev.,}qkjudge.kisen.one` のまま、k8s secret と GitHub webhook 側 token も
  不整合) → URL を `qkjudge-api-stg.kisen.one` / `qkjudge-api.kisen.one` に修正 + secret を
  両側で再同期。webhook 経由で staging に問題 5 件 (`tqk` 由来) が register された後、
  fresh user で AC submission (id=3, `result: "AC"`)、CE submission (id=4, `result: "CE"`)
  を確認。古い試行 (id=1,2) の `UE 403` は round 2 で直した `statusCode` 復元修正の
  恩恵で正しい HTTP ステータスがそのまま保存されている (旧コードなら `UE 400` に潰れる)。
  詳細は `docs/MIGRATION.md` の付録「2026-06-16 mirror incident と webhook 後始末」参照。
