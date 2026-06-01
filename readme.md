# QK Judge

休憩ジャッジ

競プロ問題公開したい！

コンパイルや実行部分はいったん外部アプリに任せる

サイト(WIP): [QK Judge](https://judge.tqk.blue)

サーバー(:koko:): [qkjudge](https://github.com/tqkoh/qkjudge)<br>
クライアント: [qkjudge-UI](https://github.com/tqkoh/qkjudge-UI)<br>
問題: [qkjudge-problems](https://github.com/tqkoh/qkjudge-problems)<br>

[API 予定](https://apis.tqk.blue/)

[開発メモ](https://github.com/tqkoh/qkjudge/blob/dev/memo.md)

## 開発セットアップ

ツールは [mise](https://mise.jdx.dev/) で管理している (`mise.toml`)。Rust ツールチェイン
(cargo / clippy / rustfmt) は別途必要。

```sh
mise install        # gitleaks / prek を入れる
prek install        # git フック (pre-commit / pre-push) を有効化
```

### コミット前チェック (prek)

[prek](https://github.com/j178/prek) (pre-commit 互換) で `.pre-commit-config.yaml` のフックを走らせる。
`prek install` 後はコミット時に自動で実行される。

- **commit 時**: trailing-whitespace / end-of-file-fixer / check-added-large-files / check-yaml /
  check-merge-conflict、`cargo fmt` (差分は自動修正 → 再 add)、**gitleaks** (secret 誤コミット防止)。
- **push 時**: `cargo clippy -D warnings` を回す予定。sqlx offline 化 (TASK-002) は完了し
  `SQLX_OFFLINE=true` で DB 無しにコンパイルできるようになったが、`-D warnings` 適用には既存の
  lint 警告 (約20件) の解消が要るため、当面は無効のまま (CI / TASK-004 で整理予定)。

全ファイルに手動で走らせる:

```sh
prek run --all-files
```

> secret (DB creds / JDoodle / tunnel token / kubeconfig 等) はコミットしない。gitleaks が
> ステージ済み差分を走査して検知・ブロックする。ドキュメント用のサンプル値は
> `# gitleaks:allow` コメントか `.gitleaksignore` で個別に許可する。

## ローカル実行 (docker compose)

旧本番と同等の主要フロー (signup / login / problems / submit→judge / submissions) を手元で再現する。
`app` (Rust, マルチステージビルド) + `mariadb:11.4` (初期スキーマ `migrations/v1.2.3.sql` を投入)。

```sh
cp .env.example .env   # 値を埋める (.env は gitignore。secret を含めるのでコミットしない)
docker compose up --build
```

- アプリは `http://localhost:8080` で待ち受ける (`/ping` が 200 を返す)。
- `mariadb` は手元の別 DB との衝突を避けるため host `3307` に publish している (`mysql -h127.0.0.1 -P3307`)。
- 起動時に `kisepichu/qkjudge-problems` の `dist` を `/data/problems` へ clone (既存なら pull) する。
- 主な環境変数: `CORS_ALLOW_ORIGIN` (1 環境 1 オリジン)、`SESSION_KEY` (64 hex。未設定だと再起動で
  ログアウト)、`COMPILER_API_CLIENT_ID`/`_SECRET` (JDoodle。提出のジャッジに必須)。詳細は `.env.example`。

> DB 無しでもイメージはビルドできる (sqlx offline)。`sqlx::query!` の検証メタデータは
> `sqlx-data.json` にキャッシュ済みで、ビルドは `SQLX_OFFLINE=true` で走る。スキーマやクエリを
> 変更したら、DB を起動して `DATABASE_URL=mysql://qkjudge:...@127.0.0.1:3307/qkjudge cargo sqlx prepare`
> で再生成しコミットする。
