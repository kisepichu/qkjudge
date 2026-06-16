# TASK-007: JDoodle CE 誤判定の修正 (Phase 6 / 本命)

## 参照

- `docs/MIGRATION.md`「本来の目的」
- `src/routes/post_submit.rs` (`CompilerApiResponse`、判定ロジック、`judge()`)
- `src/routes/get_execute.rs` (同 API への credit-spent 呼び出し、別の `CompilerApiResponse` 定義)
- `src/languages.rs` (C++ = id 0 / `language_code "cpp17"` / `version_index "1"`)

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

- **要キャプチャ**: 現行 JDoodle レスポンスの実物 (成功時 + わざとコンパイルエラー時) を 1 つずつ。
  資格情報はユーザーが保持。例:
  `curl -s https://api.jdoodle.com/v1/execute -H 'Content-Type: application/json' \
    -d '{"clientId":"…","clientSecret":"…","script":"…","language":"cpp17","versionIndex":"1","stdin":"6 7"}'`
  成功 / CE 両方の JSON を見て、CE を判別する**新しいシグナル**を決める
  (コンパイルエラー時のフィールド/`output` 文言/`statusCode` の差など)。
- `cpu_time == "-1"` を CE 判定に使うのをやめる。
  - 挙動の正確な把握 (現状コード): cpuTime 欠落 → `unwrap_or("-1")` → :175 `"-1".parse::<f64>()` は `-1.0` で**成功し panic しない** →
    TLE 判定 (`time_limit < -1.0`) は false → :193 `else if cpu_time == "-1"` の **CE 分岐に吸い込まれる**のが実バグ。
  - **真の panic リスクは限定的**: cpuTime が「数値でない文字列」で返ってきた場合のみ :175 `parse::<f64>().unwrap()` が panic する。
    新レスポンス確認後、ここは `unwrap_or` 等で安全化しておくと堅い (欠落バグの直接原因ではないが保険)。
  - `cpuTime` 欠落を「実行はされた / 時間は不明」として扱い、CE は別シグナル (CE 時のレスポンス形) で判定し直す。
- `get_execute.rs` 側の `CompilerApiResponse` も整合させるか確認。
- 影響範囲: 判定の各分岐 (TLE/OLE/CE/MLE/RE/WA/AC) を、現行レスポンスで一通り検証。

## チェックリスト

- [ ] 現行 JDoodle レスポンス (成功 / CE) をキャプチャし `docs/` か本タスクに記録
- [ ] 判定ロジックを現行仕様に合わせて修正: CE 判定を `cpu_time == "-1"` から CE 時の実シグナルへ差し替え
- [ ] :175 `cpu_time.parse::<f64>().unwrap()` を安全化 (cpuTime が非数値文字列でも panic しない保険)
- [ ] 正しい C++ A+B が AC、わざとコンパイルエラーが CE、WA/TLE 等も妥当になることを staging で確認
- [ ] `cargo test`

## 完了条件

- [ ] 正しい提出が AC になる (CE 誤判定が解消)
- [ ] 実際のコンパイルエラーは CE と判定される
- [ ] cpuTime が非数値文字列でも :175 で panic しない

## 作業ログ

- 2026-05-31: タスク生成。根本原因を旧 API 実データで確定。現行レスポンスのキャプチャ待ち。
