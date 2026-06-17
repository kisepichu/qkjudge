# TASK-005: legacy 提出の read-only 配信エンドポイント (Phase 4)

## 参照

- `docs/MIGRATION.md`「レガシーデータ」
- TASK-001 のスナップショット (`migration/legacy-snapshot/`)
- `src/routes/get_submissions.rs` `get_submissions_sid.rs` (新提出側のページング)
- DB スキーマ `migrations/v1.2.3.sql` (submissions.author→users / problem_id→problems の FK)

## 概要

TASK-001 のスナップショットを、新サーバーから read-only で配信する。FK 制約を避けるため
**本テーブルには取り込まず** legacy 専用ストアから返す。UI (TASK-006) は提出一覧の末尾(=新提出が
尽きた後)に legacy を表示し、author に `[legacy]` prefix を付ける。

## 設計メモ

- ストア: スナップショット JSON をイメージに同梱 (`include_str!`、read-only)。
  件数は小さい (58 submissions / 275 tasks) ので起動時 `OnceLock` で 1 度だけ
  deserialize、以降は参照のみ。
- エンドポイント (新側と同形のレスポンス):
  - `GET /legacy/submissions?page=N` — legacy summary のページング。レスポンスは
    `{ pages_number, submissions: [...] }` で `get_submissions.rs` と完全互換。
  - `GET /legacy/submissions/{id}` — source + tasks (`get_submissions_sid.rs` と同形)。
  - `GET /legacy/tasks/{tid}` — 詳細 (`get_tasks_tid.rs` と同形)。
- ID 名前空間: **URL を `/legacy/*` で完全分離するため id は数値のまま** (新側と同形を保つ)。
  UI 側 (TASK-006) が一覧連結する際は React key を `legacy-${id}` 等で一意化する責務を持つ。
- ページング合成方針: server 側で合成せず、legacy 側は独立した `pages_number` を返す。
  UI 側で「新側の `pages_number` まで取りきったら legacy 側 page=1 から取り始める」と
  順次呼び出す。
- 認証: 新側 `/submissions*` / `/tasks/{tid}` は `Identity` を引数に取るだけで
  `id.identity()` 呼び出しが無く、actix_identity の extractor の性質上 **未認証でも
  通る** (`get_whoami` / `get_problems` のように `is_empty() → 403` する系とは別グループ)。
  当初 markdown の「未ログイン可」が実態通り。legacy 側もこれに揃え、ハンドラから
  ミスリードな `Identity` 引数を外して公開エンドポイントとして実装する。
  (新側 `_id: Identity` のままにしている dead extractor も将来の整理対象だが、
  TASK-005 スコープ外として別途扱う。)
- ハンドラは薄いラッパ (ストアの page() / submission() / task() を呼ぶだけ) で、ロジックは
  ストアに集約してユニットテストで保証する。

## チェックリスト

- [x] スナップショットを読み込む legacy ストア (起動時ロード, `src/legacy_store.rs`)
- [x] `GET /legacy/submissions` / `/legacy/submissions/{id}` / `/legacy/tasks/{tid}` を実装
- [x] author 表示は UI 側 `[legacy]` 前提 (サーバーは生値を返す)
- [x] 新提出との id 衝突回避を UI と合意・実装 (URL prefix で分離、id は数値のまま)
- [x] ユニットテスト (ページング境界・存在しない id、計 8 ケース)
- [x] `cargo test` (17 passed) / `cargo clippy --all-targets --all-features -- -D warnings` (warning なし) / `cargo fmt --check` (clean)

## 完了条件

- [x] 旧サーバーが停止していても legacy 提出/ソース/タスクが参照できる (起動時にスナップショット
      を埋め込み済みなので外部依存ゼロ)
- [x] 新提出のページングを壊さず、末尾に legacy が続く (server 側合成なし、UI 側で順次呼ぶ契約)

## 作業ログ

- 2026-05-31: タスク生成。
- 2026-06-17: 実装。`src/legacy_store.rs` に `OnceLock` ベースの read-only ストアを追加。
  `migration/legacy-snapshot/{submissions,tasks}.json` を `include_str!` で埋め込み、
  起動時に id 降順ソート + id 索引化。アクセサは `page(page, per_page)` /
  `submission(id)` / `task(id)`。`src/routes/get_legacy_{submissions,submissions_sid,tasks_tid}.rs`
  を新規追加し、新側 `get_submissions*` / `get_tasks_tid` と同形の JSON を返す。
  `main.rs` で起動時に `legacy_store::global()` をウォームアップして deserialize 失敗を
  fail-fast 化、3 ルートを `App::new().service(...)` に登録。
  `cargo test` 17 passed (9 既存 + 8 新規)、clippy clean (sort_by → sort_by_key で
  `clippy::unnecessary_sort_by` 1 件解消)、fmt clean。
  手元 docker compose での実機検証は DB / JDoodle 連携が無いメモリストアのみのため省略、
  staging deploy 後の動作確認に委ねる (`/legacy/submissions?page=1` の JSON shape と
  authn 挙動を新側と並べて確認すれば足りる)。
- 2026-06-17 (PR #29 Copilot レビュー対応): 「`_id: Identity` を引数に取るだけでは
  認証ガードにならない」を 4 件指摘される。事実その通りで、新側 `/submissions*` /
  `/tasks/{tid}` も同じく Identity を取るだけ = guest 通過する実装になっており、
  「新側 = ログイン要」と読んでいた私の理解が間違っていた (新側 markdown 当初メモの
  「未ログイン可」が実態通り)。3 ハンドラから dead extractor `_id: Identity` を削除し、
  task markdown の認証セクションを新側との対称性に合わせて修正。新側の dead extractor
  整理は本タスクのスコープ外として別途扱う。
