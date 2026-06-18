---
name: migrate-task
description: qkjudge 鯖移行 (traP → leafeon) の TASK-NNN を進める手順。「TASK-00x やって」「次のタスク進めて」などで使用。docs/MIGRATION.md と tasks/ がハブ。
---

# qkjudge migrate-task ワークフロー

`docs/MIGRATION.md` の鯖移行 (traP NeoShowcase → 自宅 leafeon) を、`tasks/` をハブに 1 TASK ずつ進める。
TASK-000 (repo hygiene) で確立した進め方を踏襲する。

## 全体像

- 各 TASK は `dev` から `migrate/leafeon-TNNN-<slug>` を切り、PR base = `dev` で 1 タスク 1 PR。
  (旧 `migrate/leafeon` epic ブランチは PR #24 で dev に統合済み、今は廃止。
   ブランチ命名のみ `migrate/leafeon-*` を継承する。)
- `dev` への merge 後、staging で実機検証 → 区切りで `dev → master` を別 PR。
- **secret / private URL / ホスト固有パスはコミットしない** (gitleaks が検知)。commit/push 前は確認する。
- **`dev` / `master` は branch protection で `allow_force_pushes: false`, `enforce_admins: true`**。
  force-push 系の destructive 操作は使わない (旧 traP gitea mirror の force-push を弾くため必要)。

## 手順

1. 対象 TASK ファイル (`tasks/todo/TASK-NNN-*.md`) と `docs/MIGRATION.md` の該当行を読む。
2. ブランチを切る前に `git switch dev && git pull` で最新化。
3. `git switch -c migrate/leafeon-TNNN-<slug>` (dev 起点)。
4. タスクを doing へ: `git mv tasks/todo/TASK-NNN-*.md tasks/doing/`。
5. チェックリストを上から進める。各項目の成果物と検証方法を明記する (非実装作業が多く TDD は使えないことが多い)。
   - **判断・検証はオーケストレータ (このセッション) が担う**: 設計判断、secret 検証、依存関係の発見など。
   - **無監督で回せる定型・機械作業は headless subagent に offload する** (下記)。
6. 完了したら検証結果をタスクの作業ログに記録し、チェックを `[x]` に更新、`tasks/done/` へ `git mv`。
7. `/commit` → `/pr` (base=`dev`) → `/pr-review` まで進める。

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

## 確立済みの環境メモ

- ツールは `mise.toml` でピン留め (gitleaks / prek)。Rust ツールチェインはグローバル mise 管理。
- prek (pre-commit 互換): commit 時に fmt / 汎用 / gitleaks、**push 時に clippy** (`SQLX_OFFLINE=true cargo clippy --all-targets --all-features -- -D warnings`)。
  TASK-002 で `sqlx-data.json` をコミット済みなので DB なしでビルド可。CI (TASK-004) でも同等を回す。
- staging への deploy は dev push で auto (TASK-004)。staging API host は
  `qkjudge-api-stg.kisen.one` (Cloudflare Universal SSL 1 階層制約のため `*.dev.kisen.one`
  系は使えない、PR #23 参照)。
- DB スキーマには `submissions.author → users` / `problem_id → problems` の FK が効いている
  (`migrations/v1.2.3.sql`)。legacy データを新 DB に insert すると確実に弾かれるので、
  legacy 配信は別ストアで行う (TASK-005)。
