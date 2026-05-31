# TASK-000: リポジトリ衛生 / pre-commit (prek) 導入 (Phase 0)

## 参照

- `docs/MIGRATION.md`「ブランチ運用」(secret をコミットしない方針)
- prek (pre-commit 互換のフックランナー)

## 概要

secret を扱い始める **TASK-002 以降の前提**として、コミット前チェックを自動化する。特に**secret 誤コミット防止**が重要
(本移行は DB creds / JDoodle / tunnel token / kubeconfig を扱う)。prek で pre-commit フックを導入する。

> 着手順: TASK-001(スナップショット) は公開 API のみ・秘密情報なしのため、本タスクより先でも安全。
> 旧サーバー停止リスクを優先するなら TASK-001 を先行してよい (MIGRATION.md「着手順について」)。

## 設計メモ

- ランナー: prek (`.pre-commit-config.yaml` 互換)。導入手順を README/docs に記載。
- 想定フック (要相談・調整):
  - Rust: `cargo fmt --check`、`cargo clippy -- -D warnings` (重いのでローカルは fmt のみ、clippy は CI に寄せる案も)
  - 汎用: trailing-whitespace / end-of-file-fixer / check-added-large-files / check-yaml / check-merge-conflict
  - **secret 検出**: gitleaks か detect-secrets (誤コミット防止の主目的)
- CI(TASK-004) でも同等チェックを回し、ローカルすり抜けを防ぐ。

## チェックリスト

- [x] フック構成を確定 (上記から取捨選択)
- [x] `.pre-commit-config.yaml` 追加、prek 導入手順を docs 化 (readme.md「開発セットアップ」)
- [x] secret 検出フックが DB creds / token 類のダミーを検知することを確認
- [x] 既存ファイルに対して一度全フックを通し、フォーマット差分を解消 (memo.md の末尾空白のみ)

## 確定したフック構成

- **secret 検出**: gitleaks (mise で導入)。`gitleaks git --pre-commit --redact --staged --verbose`。
- **汎用**: trailing-whitespace / end-of-file-fixer / check-added-large-files / check-yaml /
  check-merge-conflict (pre-commit/pre-commit-hooks v5.0.0)。
- **Rust 整形**: `cargo fmt --all`、commit 時・自動修正してブロック。
- **clippy**: pre-push 予定だが、`sqlx::query!` がコンパイルに DB/sqlx offline を要するため
  **TASK-002 まで無効化** (config にコメントアウト + TODO)。CI (TASK-004) でも回す。
- ツールは `mise.toml` (gitleaks 8.30.1 / prek 0.4.1) でピン留め。

## 完了条件

- [x] コミット時に fmt / 汎用 / secret 検出フックが走る
- [x] secret 誤コミットがフックで止まる (非 example トークンを `generic-api-key` で検知・非ゼロ終了)

## 作業ログ

- 2026-05-31: タスク生成 (MIGRATION.md レビューコメント「TASK-001 の前に prek 的なの」より)。
- 2026-06-01: `mise.toml` (gitleaks/prek)・`.pre-commit-config.yaml`・readme 開発セットアップを追加。
  prek 全ファイル走査の機械作業は headless subagent (CLAUDE_CONFIG_DIR=$CLAUDE_OFFLOAD_CONFIG_DIR) に offload。
  gitleaks のブロックを自前検証 (example キーは既定 allowlist で正しくスルー、非 example は検知)。
  clippy は sqlx offline (TASK-002) 依存のためコメントアウトで保留。
