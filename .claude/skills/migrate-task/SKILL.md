---
name: migrate-task
description: qkjudge 鯖移行 (migrate/leafeon epic) の TASK-NNN を進める手順。「TASK-00x やって」「次のタスク進めて」などで使用。docs/MIGRATION.md と tasks/ がハブ。
---

# qkjudge migrate-task ワークフロー

`docs/MIGRATION.md` の鯖移行 (traP NeoShowcase → 自宅 leafeon) を、`tasks/` をハブに 1 TASK ずつ進める。
TASK-000 (repo hygiene) で確立した進め方を踏襲する。

## 全体像

- epic ブランチ: `migrate/leafeon` (dev から分岐)。
- 各 TASK は `migrate/leafeon` から `migrate/leafeon-TNNN-<slug>` を切り、PR → `migrate/leafeon`。
- 全 TASK 完了後に `migrate/leafeon` → `dev` を PR。
- **secret / private URL / ホスト固有パスはコミットしない** (gitleaks が検知)。commit/push 前は確認する。

## 手順

1. 対象 TASK ファイル (`tasks/todo/TASK-NNN-*.md`) と `docs/MIGRATION.md` の該当行を読む。
2. ブランチを切る: `git switch -c migrate/leafeon-TNNN-<slug>` (epic ブランチ起点)。
3. タスクを doing へ: `git mv tasks/todo/TASK-NNN-*.md tasks/doing/`。
4. チェックリストを上から進める。各項目の成果物と検証方法を明記する (非実装作業が多く TDD は使えないことが多い)。
   - **判断・検証はオーケストレータ (このセッション) が担う**: 設計判断、secret 検証、依存関係の発見など。
   - **無監督で回せる定型・機械作業は headless subagent に offload する** (下記)。
5. 完了したら検証結果をタスクの作業ログに記録し、チェックを `[x]` に更新、`tasks/done/` へ `git mv`。
6. `/commit` → `/pr` (base=`migrate/leafeon`) → `/pr-review` まで進める。

## offload (headless subagent)

機械作業 (フック全走査と自動修正、一括整形、定型生成など) は別アカウントの headless claude に逃がす。
生成コストは offload 先に課金され、オーケストレータは「プロンプト生成 + レポート取り込み + 裏取り」だけ負担する。

offload 先の config ディレクトリは環境変数 `CLAUDE_OFFLOAD_CONFIG_DIR` で渡す (command にハードコードしない)。
各自が自分の別アカウント config dir を `export CLAUDE_OFFLOAD_CONFIG_DIR=~/.claude-<sub>` のように設定しておく。

```sh
env CLAUDE_CONFIG_DIR="$CLAUDE_OFFLOAD_CONFIG_DIR" \
  claude -p "$(cat /tmp/<task>-prompt.md)" \
  --permission-mode acceptEdits \
  --allowedTools "Bash(mise:*)" "Bash(cargo:*)" "Bash(prek:*)" "Bash(git status*)" "Edit" "Write" \
  --output-format text < /dev/null
```

- cwd は repo ルート。subagent はこの作業ツリーを直接編集し、結果はそのまま残る。
- `--permission-mode acceptEdits` + 必要な `--allowedTools` を明示。`--dangerously-skip-permissions` /
  `bypassPermissions` は使わない (managed-settings 環境では逆に何も通らなくなる)。
- ツールは mise 提供なので worker には `mise exec -- <tool>` を使わせる。
- worker には **commit/push/ブランチ操作を禁止**し、変更は作業ツリーに残させてオーケストレータがレビューする。
- プロンプトファイルは `/tmp` に置き repo に持ち込まない。
- `$CLAUDE_OFFLOAD_CONFIG_DIR` が未設定なら Agent (Task) ツールの general-purpose subagent で
  セッション内実行にフォールバックする。

## 確立済みの環境メモ (TASK-000)

- ツールは `mise.toml` でピン留め (gitleaks / prek)。Rust ツールチェインはグローバル mise 管理。
- prek (pre-commit 互換) で commit 時に fmt / 汎用 / gitleaks が走る。`prek install` でフック有効化。
- **clippy は `sqlx::query!` がコンパイルに DB/sqlx offline を要するため TASK-002 完了まで無効化**
  (`.pre-commit-config.yaml` にコメントアウト + TODO)。CI (TASK-004) でも回す。
