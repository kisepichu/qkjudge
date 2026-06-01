# TASK-002: コンテナ化準備 (Phase 1)

## 参照

- `docs/MIGRATION.md`「ターゲット構成」「Cookie/CORS」
- `src/main.rs` (bind / CORS ハードコード / cookie private_key)
- `showcase.yaml` (旧起動: binary chmod → problems clone → 実行)
- `Cargo.toml` (sqlx 0.6, features mysql) / `memo.md` (DB 無いとビルド不可)

## 概要

アプリを leafeon の k3s で動かす前提を整える。手元 docker compose で**現状パリティ**(signup/login/
problems/submit→judge/submissions) が動く状態をゴールにする。

## 設計メモ

- **sqlx offline**: `query!`/`query_as!` がビルド時に実 DB を要求する。`cargo sqlx prepare` で
  `.sqlx/` を生成しコミット、ビルドは `SQLX_OFFLINE=true`。prepare には一時的に MariaDB が要る
  (compose の DB を使う)。
- **Dockerfile (マルチステージ)**: builder(rust, `SQLX_OFFLINE=true` で `cargo build --release`) →
  runtime(debian-slim, `git` 同梱: problems clone/pull に必要)。`target/release/qkjudge` の
  git コミットは廃止し `.gitignore` から除外 (Docker ビルドへ移行)。`.dockerignore` 追加。
- **entrypoint**: 起動時に `kisepichu/qkjudge-problems` の `dist` を `PROBLEMS_REPO_ROOT` に clone
  (存在すれば pull)、その後バイナリ実行。problems は emptyDir/PVC を想定 (TASK-003 で接続)。
- **main.rs 変更**:
  - CORS `Access-Control-Allow-Origin` を env (`CORS_ALLOW_ORIGIN`) 化 (ハードコード `judge.tqk.blue` 撤廃)。
    - 注意: `Allow-Credentials: true` 併用時は `Access-Control-Allow-Origin` に**ワイルドカード不可・単一値のみ**。
    - 方針 (要選択): (a) **1 環境 1 オリジンで割り切る** (prod/staging を overlay の env で分離。最小実装) か、
      (b) **カンマ区切り env + リクエスト `Origin` 照合**で動的に許可オリジンを echo する (ローカルから staging を叩く等に対応)。
      デフォルトは (a)、必要になれば (b) に拡張。
  - cookie `private_key` を env (`SESSION_KEY`、64 hex) から読む。未設定時のみランダム生成 (開発用)。
    → 再起動で全員ログアウトする現象を解消。
  - cookie を `SameSite=Lax` に (フロント/ API を kisen.one 同一サイトのサブドメインに置く前提)。`Secure` は維持。
- **MariaDB バージョン固定**: compose で `mariadb:<pin>` を固定 (旧 10.1 / 手元 10.6 で SQL 差異の前科)。
  既存クエリ (`last_insert_id()` 使用、RETURNING 不使用) が通るバージョンを選定。
- **compose**: `app` + `mariadb`(初期スキーマ `migrations/v1.2.3.sql` を初期化投入) + env_file(gitignore)。
  - 補足: `migrations/` は **mysqldef(宣言的スキーマ) + `migrate.sh`** 運用。`v1.2.3.sql` は増分 DDL ではなく
    **「望ましい状態」の全 5 テーブルの CREATE TABLE**(plain DDL、`migrations` テーブルも含む)。空 DB へ
    `mysql < v1.2.3.sql` で初期化できる。`problems` に `AUTO_INCREMENT=5` がベタ書きされている点に注意
    (新規環境では実害ないが把握しておく)。将来 sqlx-cli migrations への移行も検討余地。
- **旧 Showcase 設定の整理**: コミット済みバイナリ廃止に伴い、旧 `showcase.yaml`(NeoShowcase 用) は不要になる。
  残しても実害は薄いが、移行の整理として削除するか「歴史的経緯として保持」を判断・明記する
  (起動手順・problems clone・cname 等の情報は本タスク/MIGRATION.md 側に移してから消すのが安全)。
- 既存ユニットテスト (`post_fetch_problems` の HMAC テスト) が通ること。

## チェックリスト

### sqlx offline
- [x] `cargo sqlx prepare` で `sqlx-data.json` 生成・コミット、`SQLX_OFFLINE=true` でビルド確認
  (sqlx 0.6 のためキャッシュは `.sqlx/` ではなく単一 `sqlx-data.json`。`offline` feature を有効化)
### main.rs
- [x] CORS オリジンを env 化 (`CORS_ALLOW_ORIGIN`) / cookie key を env 化 (`SESSION_KEY`、未設定時ランダム) / `SameSite=Lax`
### Docker
- [x] マルチステージ Dockerfile + entrypoint (problems clone/pull→実行) + `.dockerignore`
- [x] コミット済みバイナリ廃止 (`.gitignore` 調整、`target/release/qkjudge` を追跡から除外)
### compose / 検証
- [x] `mariadb:11.4` 固定 + 初期スキーマ投入 (`v1.2.3.sql`) + env_file(`.env` gitignore、`.env.example` 提供)
- [x] `docker compose up` で `/ping` 200、signup/login、problems(fetch 後)、submit→judge dispatch、submissions を手動確認
- [x] `cargo test` がパス (HMAC テスト 2 件)

## 完了条件

- [x] DB 無しでもイメージがビルドできる (sqlx offline。Docker builder stage で `SQLX_OFFLINE=true` ビルド成功)
- [x] compose で旧本番と同等の主要フローが手元で動く
- [x] secret はコミットされていない (`.env` は gitignore、`sqlx-data.json` はクエリメタデータのみ)

## 作業ログ

- 2026-05-31: タスク生成。
- 2026-06-01: 実装・検証完了。
  - **設計判断** (ユーザー確認): MariaDB pin = **11.4 LTS**、CORS = **(a) 1 環境 1 オリジン** (`CORS_ALLOW_ORIGIN` env)、
    `showcase.yaml` = **削除** (内容は `docs/MIGRATION.md` 付録へ移管)。
  - **sqlx offline**: sqlx 0.6 のオフラインは `.sqlx/` ディレクトリではなく単一 `sqlx-data.json`。
    `Cargo.toml` の sqlx features に `offline` を追加。`sqlx-cli ^0.6` を入れ、compose の MariaDB
    (host 3307 に publish) に対し `cargo sqlx prepare` で生成。`env -u DATABASE_URL SQLX_OFFLINE=true cargo check`
    で DB 無しビルドを確認。
  - **main.rs**: `Access-Control-Allow-Origin` を env 化 (`judge.tqk.blue` ハードコード撤廃)。cookie 署名鍵を
    `SESSION_KEY` (hex, 32byte 以上を assert) から読み、未設定時のみランダム生成 (warn ログ)。
    `SameSite::None` → `SameSite::Lax`、`Secure` 維持。
  - **Docker**: `rust:1.96-bookworm` builder (buildpack-deps 同梱の openssl/pkg-config を使い apt 不要) →
    `debian:bookworm-slim` runtime (git/ca-certificates/libssl3)。`docker-entrypoint.sh` が
    `kisepichu/qkjudge-problems` の `dist` を `/data/problems` へ clone/pull してからバイナリ実行。
    `.dockerignore` 追加。`target/release/qkjudge` を `git rm --cached` で追跡解除、`.gitignore` を `/target` に統一。
  - **PROBLEMS_ROOT のスラッシュ**: コードは `PROBLEMS_ROOT + 問題名` を素朴に連結するため末尾スラッシュ必須。
    `PROBLEMS_ROOT=/data/problems/` (末尾 `/`)、`PROBLEMS_REPO_ROOT=/data/problems` (git 操作先) と分けた。
    当初スラッシュ無しで fetch が 500 になり判明 → 修正後 fetch 204・problems 反映・submit のテストケース
    パス解決 (`/data/problems/a_plus_b/out/...`) を確認。
  - **検証 (compose)**: `/ping`=200、signup=201、login=200 (`Set-Cookie: …; SameSite=Lax; Secure; HttpOnly`、
    `Access-Control-Allow-Origin: http://localhost:3000` + `…-Credentials: true` を確認)、whoami=200、
    fetch/problems=204、problems 一覧反映、submit=200 (submission insert + テストケース解決 + judge dispatch まで)、
    submissions=200。`cargo test` パス。
  - **未検証 (環境依存)**: submit→judge の **最終判定**は外部 JDoodle (`COMPILER_API_CLIENT_ID/_SECRET`) が要るため
    手元では判定結果まで確認できず (提出〜judge dispatch までは確認済み)。
  - **補足**: `Cargo.lock` は cargo 1.96 によりロック形式 v3→v4 に更新 (新規依存追加なし)。
    prek の clippy フックは sqlx offline 化で DB 不要になったが、`-D warnings` 化には既存 lint 約20件の
    解消が必要なため当面無効のまま (CI/TASK-004 で対応予定) — コメントを更新。
