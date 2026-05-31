# TASK-001: レガシーデータのスナップショット取得 (Phase 0 / 保険)

## 参照

- `docs/MIGRATION.md`「失われたもの」「レガシーデータ」
- 旧サーバー公開 API: `https://tqk.trap.show/qkjudge`
- ルート: `src/routes/get_submissions.rs` `get_submissions_sid.rs` `get_tasks_tid.rs`
  `get_problems.rs` `get_problems_pid.rs`(いずれも未ログインで応答する)

## 概要

旧サーバーがいつ停止されても旧データ(提出履歴・ソース・テストケース結果・問題文)を失わないよう、
公開 API を全巡回して JSON スナップショットを `migration/legacy-snapshot/` に保存するワンオフ
スクリプトを作る。**最優先**(時限式)。本テーブルへ取り込まず、後段 (TASK-005) で legacy 専用
配信に使う素材とする。

## 設計メモ

- 言語: Python3 (標準ライブラリのみ、追加依存なし)。リトライ + レート配慮 (sleep)。
- 取得対象:
  - `GET /problems` → 一覧、各 `GET /problems/{id}` → statement 含む詳細。
  - `GET /submissions?page=1..pages_number` で全 submission summary を列挙
    (`pages_number` はレスポンスに含まれる)。
  - 各 `GET /submissions/{id}` → source + tasks(id,result)。
  - 各 task の `GET /tasks/{tid}` → input/output/expected/result/memory/cpu_time。
- 出力: `problems.json` / `submissions.json` (詳細を内包) / `tasks.json` / `meta.json`(取得日時・件数・base URL)。
  生レスポンスも `raw/` に保存して再加工可能にする。
- 冪等: 既取得分はスキップ可能にし、途中再開できる。
- スナップショットは**コミットして残す** (legacy データの最終バックアップ)。秘密情報は含まない (公開 API)。

## チェックリスト

- [ ] `migration/legacy-snapshot/scrape.py` を作成 (上記取得・保存・リトライ・再開)
- [ ] `migration/legacy-snapshot/README.md` に実行方法と取得日時・件数を記録
- [ ] 実行して `problems / submissions / tasks` を全件取得
- [ ] 件数の妥当性を確認 (submissions の `pages_number` × ~10 と一致、tasks が submission に紐づく)
- [ ] スナップショット JSON をコミット

## 完了条件

- [ ] 旧サーバーが停止しても、全 submission の source と全 task の入出力/結果が手元 JSON に残っている
- [ ] スクリプトは再実行で安全 (再開可能・公開 API のみ)

## 作業ログ

- 2026-05-31: タスク生成。旧 API の応答形状を確認済 (例: task 321 = output "13"/cpu_time "-1"/CE、task 308 = cpu_time "0.00"/AC)。submissions は `pages_number:6`。
