# legacy-snapshot — 旧 qkjudge データのスナップショット

旧サーバー (traP NeoShowcase, `https://tqk.trap.show/qkjudge`) はいつ停止されても
おかしくないため、公開 API を全巡回して提出履歴・ソース・テストケース結果・問題文を
JSON で保存したもの。本テーブルへは取り込まず、後段 (TASK-005) の legacy 専用配信に使う
素材とする (詳細は `docs/MIGRATION.md`)。

公開 API のみを叩いているため秘密情報は含まない。このディレクトリ一式は legacy データの
最終バックアップとしてコミットして残す。

## 取得結果 (最新実行)

- base URL: `https://tqk.trap.show/qkjudge`
- 取得日時: 2026-06-01 JST (2026-05-31T17:29 UTC)
- 件数:
  - problems: 5
  - submissions: 58 (`pages_number` = 6, 1 ページ 10 件で最終ページのみ 8 件)
  - tasks: 275 (全 submission 詳細が宣言する task id と完全一致、孤児なし)

## ファイル

| ファイル           | 内容                                                                  |
| ------------------ | --------------------------------------------------------------------- |
| `problems.json`    | 問題詳細 (statement 含む) の配列                                      |
| `submissions.json` | 提出詳細 (source + 各 task の id/result) の配列                       |
| `tasks.json`       | task 詳細 (input/output/expected/result/memory/cpu_time) の配列      |
| `meta.json`        | base URL / 取得日時 / 件数                                            |
| `raw/`             | 各エンドポイントの生レスポンス (再加工・再開用)                       |

## 実行方法

標準ライブラリのみ。追加依存なし。

```sh
python3 migration/legacy-snapshot/scrape.py
```

- **冪等 / 再開可能**: 生レスポンスを `raw/` に保存し、既取得分は再 fetch せずそれを読む。
  途中で落ちても再実行すれば続きから取得する。全件キャッシュ済みなら数秒で完了する。
- 再取得し直したい場合は対象の `raw/` ファイル (またはディレクトリ全体) を消してから実行する。
- リトライ + レート配慮 (各リクエスト間 sleep, 失敗時は指数バックオフ)。

### 環境変数 (任意)

| 変数                 | 既定                          | 説明                       |
| -------------------- | ----------------------------- | -------------------------- |
| `QKJUDGE_LEGACY_BASE`| `https://tqk.trap.show/qkjudge` | base URL                 |
| `QKJUDGE_SLEEP`      | `0.3`                         | リクエスト間 sleep (秒)    |
| `QKJUDGE_RETRIES`    | `5`                           | 最大リトライ回数           |
| `QKJUDGE_TIMEOUT`    | `30`                          | リクエストタイムアウト (秒)|

## 取得対象 (公開 API、いずれも未ログインで応答)

- `GET /problems` → 一覧 → 各 `GET /problems/{id}` (statement 含む)
- `GET /submissions?page=1..pages_number` → 全 summary 列挙
- 各 `GET /submissions/{id}` → source + tasks(id, result)
- 各 task の `GET /tasks/{tid}` → input/output/expected/result/memory/cpu_time
