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
- [ ] `cargo sqlx prepare` で `.sqlx/` 生成・コミット、`SQLX_OFFLINE=true` でビルド確認
### main.rs
- [ ] CORS オリジンを env 化 / cookie key を env 化 (未設定時ランダム) / `SameSite=Lax`
### Docker
- [ ] マルチステージ Dockerfile + entrypoint (problems clone/pull→実行) + `.dockerignore`
- [ ] コミット済みバイナリ廃止 (`.gitignore` 調整、`target/release/qkjudge` を追跡から除外)
### compose / 検証
- [ ] `mariadb:<pin>` 固定 + 初期スキーマ投入 + env_file(gitignore)
- [ ] `docker compose up` で `/ping` 200、signup/login、problems(fetch 後)、submit→judge、submissions を手動確認
- [ ] `cargo test` がパス

## 完了条件

- [ ] DB 無しでもイメージがビルドできる (sqlx offline)
- [ ] compose で旧本番と同等の主要フローが手元で動く
- [ ] secret はコミットされていない (env_file は gitignore)

## 作業ログ

- 2026-05-31: タスク生成。
