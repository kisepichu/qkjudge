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

- [ ] フック構成を確定 (上記から取捨選択)
- [ ] `.pre-commit-config.yaml` 追加、prek 導入手順を docs 化
- [ ] secret 検出フックが DB creds / token 類のダミーを検知することを確認
- [ ] 既存ファイルに対して一度全フックを通し、フォーマット差分を解消

## 完了条件

- [ ] コミット時に fmt / 汎用 / secret 検出フックが走る
- [ ] secret 誤コミットがフックで止まる

## 作業ログ

- 2026-05-31: タスク生成 (MIGRATION.md レビューコメント「TASK-001 の前に prek 的なの」より)。
