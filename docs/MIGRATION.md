# qkjudge サーバー移行計画 (traP NeoShowcase → 自宅サーバー leafeon)

## 背景

qkjudge サーバーは traP の NeoShowcase 上で、gitea `git.trap.jp/tqk/qkjudge` への push を
トリガーに自動デプロイされていた。gitea は `github.com/kisepichu/qkjudge` へ **一方向 push mirror**
していたため、GitHub への push は gitea には戻らない。

サークル卒業により traQ/SSO アカウントが無効化され、gitea (push/fetch/web)・NeoShowcase ダッシュボード
のいずれにもアクセスできず、頼める現役部員もいない。**既存の traP インフラ経由ではサーバーを
更新できない。** よって自宅サーバー leafeon へ移行する。

### まだ手中にあるもの(移行の足場)

- GitHub: `kisepichu/qkjudge`・`kisepichu/qkjudge-UI`・`kisepichu/qkjudge-problems` (以前のユーザー名
  `tqkoh` からのリダイレクトが残っている)。
- DNS: `tqk.blue`、`kisen.one` (いずれも操作可)。
- leafeon: 自宅 NixOS サーバー (SSH は VPN または Cloudflare Access 経由)。ホスト設定は別リポジトリ
  `nixos-config-private` で宣言的に管理。

### 失われたもの / 復元可能性

- **復元不能**: 旧 MariaDB の users の bcrypt パスワード。直接アクセス不可で、API でも取得できない
  → **ユーザーは再登録**。
- **復元可能 (API スナップショット経由)**: 旧 submissions / tasks (ソース・入出力・判定結果) は
  生きている旧サーバー公開 API (`https://tqk.trap.show/qkjudge`) から取得できる。旧サーバーは停止される
  可能性があるため早期にスナップショットを取る (TASK-001)。
- **復元可能 (リポジトリから)**: problems は `kisepichu/qkjudge-problems` の `dist` ブランチから。

## 目的

1. **いつでも更新できる状態を取り戻す** こと自体が第一の目的 (現状デプロイ経路が完全に断たれている)。
2. その上で **JDoodle CE 誤判定の修正** (本来やりたかった機能修正、Phase 6 / TASK-007)。
   JDoodle が成功時に `cpuTime` を返さなくなり、`post_submit.rs` の `cpu_time == "-1"` ヒューリスティック
   が全提出を CE と誤判定する (memory は今も string で取れている)。

## 決定事項

| 項目                 | 決定                                                                                                                                                                           |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| オーケストレーション | k3s on leafeon。最初から prod / staging のマルチ環境 (旧 Showcase のブランチ別デプロイを踏襲)                                                                                  |
| 環境マッピング       | `master`→prod (`qkjudge.kisen.one` / API `qkjudge-api.kisen.one`)、`dev`→staging (`dev.qkjudge.kisen.one` / `qkjudge-api-stg.kisen.one`)                                       |
| コンテナレジストリ   | GHCR (`ghcr.io/kisepichu/qkjudge`)                                                                                                                                             |
| デプロイ反映         | leafeon 上のセルフホスト GitHub Actions Runner が `kubectl` 適用 (k3s API を外部公開しない)。**セキュリティ堅牢化必須**。GitOps(Argo/Flux) は将来移行                          |
| 公開                 | cloudflared を k3s 内 Deployment として動かし、tunnel ingress 設定も k3s マニフェストで管理する (鯖機を直接設定せず pull+反映で完結)。DNS は Cloudflare に CNAME               |
| Cookie/CORS          | フロント/ API を `kisen.one` 同一 registrable domain のサブドメインに置き `SameSite=Lax; Secure`。CORS オリジンは env 化                                                       |
| 鯖機の扱い           | 鯖機を直接設定変更しない。ホスト設定の変更が要る場合 (例: k3s 導入) は `nixos-config-private` 経由で宣言的に。鯖機では pull + 反映のみ。デプロイは k3s/docker の標準手順で完結 |
| レガシーデータ       | 早期に旧公開 API を全巡回してスナップショット保存。FK 回避のため legacy 専用ストア。UI は `[legacy]` prefix で一覧末尾に表示                                                   |
| 実行基盤             | JDoodle 継続 (資格情報あり、200回/日・テストケース1件=1コール)                                                                                                                 |
| ジャッジ自前化       | 今回スコープ外 (将来検討)                                                                                                                                                      |

## ターゲット構成 (k3s)

```
prod namespace                         staging namespace
  qkjudge (deploy, image :master)        qkjudge (deploy, image :dev)
  mariadb (statefulset + PVC)            mariadb (statefulset + PVC)
  secret (JDoodle/DB/webhook/cookie)     secret (...)
  ingress qkjudge-api.kisen.one          ingress qkjudge-api-stg.kisen.one
cloudflared (deploy) ── tunnel ── Cloudflare ── qkjudge-api.kisen.one / qkjudge-api-stg.kisen.one
frontend: GitHub Pages (kisen.one 独自ドメイン) — k3s 外
```

## ブランチ運用

- epic ブランチ: `migrate/leafeon` (dev から分岐)。
- 各 TASK は `migrate/leafeon` から `migrate/leafeon-TNNN-<slug>` を切り、PR → `migrate/leafeon` にマージ。
- 全タスク完了後に `migrate/leafeon` → `dev` を PR。
- commit/push の前は必ず止まって確認する。secret/private URL/ホスト固有パスはコミットしない。
- フロント (`qkjudge-UI`)・problems (`qkjudge-problems`) は各リポジトリで PR。本計画とタスクは
  この server repo の `tasks/` をハブに管理する。

## タスク一覧

| #        | 内容                                                                            | repo       | Phase |
| -------- | ------------------------------------------------------------------------------- | ---------- | ----- |
| TASK-000 | リポジトリ衛生 / pre-commit (prek) 導入                                         | qkjudge    | 0     |
| TASK-001 | レガシースナップショット取得                                                    | qkjudge    | 0     |
| TASK-002 | コンテナ化準備 (sqlx offline / main.rs env・cookie / Dockerfile / 開発 compose) | qkjudge    | 1     |
| TASK-003 | k3s prod+staging マニフェスト (MariaDB / Ingress / cloudflared / DNS / backup)  | qkjudge    | 2     |
| TASK-004 | CI/CD (Actions→GHCR / セルフホスト Runner / セキュリティ)                       | qkjudge    | 3     |
| TASK-005 | legacy 配信エンドポイント (read-only)                                           | qkjudge    | 4     |
| TASK-006 | フロント切替 + judge.tqk.blue リダイレクト                                      | qkjudge-UI | 4-5   |
| TASK-007 | JDoodle CE 誤判定の修正 (本命)                                                  | qkjudge    | 6     |

進行は `tasks/todo` → `tasks/doing` → `tasks/done`。

## 付録: 旧 NeoShowcase 設定 (`showcase.yaml`) の対応付け

旧デプロイは `showcase.yaml` で定義していたが、コンテナ化 (TASK-002) で役割を移したため削除した。
内容と移行先は以下の通り (歴史的経緯としてここに記録):

| 旧 `showcase.yaml`                                                 | 移行先                                                          |
| ------------------------------------------------------------------ | -------------------------------------------------------------- |
| `startup`: `chmod` 済みバイナリ実行 + `git clone -b dist … problems` | `docker-entrypoint.sh` (起動時に problems を clone/pull → 実行) |
| `entrypoint: ./target/release/qkjudge`                             | Dockerfile `ENTRYPOINT` (マルチステージビルドの成果物)         |
| `http_proxy: 8080`                                                 | Dockerfile `EXPOSE 8080` / Service (TASK-003)                  |
| `https: on`                                                        | cloudflared + Ingress (TASK-003)                               |
| `use_mariadb: true`                                                | compose の `mariadb` サービス / k3s StatefulSet (TASK-003)     |
| `branch.dev.cname: dev_tqk_qkjudge.trap.games`                     | `dev`→`dev.qkjudge.kisen.one` (上記「決定事項」)               |

problems リポジトリの取得元は旧 `tqkoh/qkjudge-problems` から `kisepichu/qkjudge-problems` に更新した。
