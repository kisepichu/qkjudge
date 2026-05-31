# TASK-003: k3s prod+staging マニフェスト (Phase 2)

## 参照

- `docs/MIGRATION.md`「ターゲット構成」「環境マッピング」「公開」
- TASK-002 の Docker イメージ・env 一覧
- 前提: leafeon に k3s が入っていること。k3s 導入はホスト変更なので**鯖機を直接いじらず
  `nixos-config-private` 経由 (`services.k3s` 等) で宣言的に**行う (鯖機では pull + 反映のみ)。
  cloudflared tunnel トークンは Cloudflare 側で発行 (ユーザー操作)。

## 概要

leafeon の k3s に prod / staging の 2 環境を立てる。旧 Showcase のブランチ別デプロイ
(master=prod / dev=staging) を再現。公開は cloudflared を**クラスタ内 Deployment** として動かし、
tunnel ingress 設定も k3s マニフェストで管理する (鯖機を直接設定せず、デプロイは pull+反映で完結)。

## 設計メモ

- 配置: マニフェストは本 repo の `deploy/k3s/` に置く。`base/` + `overlays/{prod,staging}`(kustomize) で
  環境差分 (image tag / host / namespace / DB 名) を吸収。
- **MariaDB**: StatefulSet + PVC(local-path)。バージョンは TASK-002 と一致。初期スキーマ
  `migrations/v1.2.3.sql` を initContainer か initdb ConfigMap で投入 (宣言的全 CREATE。詳細は TASK-002 補足)。
- **app**: Deployment + Service。env は Secret 参照 (JDoodle clientId/secret、DB creds、
  `GITHUB_WEBHOOK_TOKEN`、`SESSION_KEY`、`CORS_ALLOW_ORIGIN`)。problems 用 PVC/emptyDir をマウント。
- **Secret 管理**: 平文をコミットしない。ローカル gitignore な値から `kubectl create secret` で投入する
  手順を README 化 (将来 Sealed Secrets / sops に移行余地)。
- **Ingress**: k3s 同梱 Traefik。`api.qkjudge.kisen.one`(prod) / `api.dev.qkjudge.kisen.one`(staging)。
- **cloudflared**: Deployment + tunnel config(ConfigMap) + token(Secret)。
  ルーティングは **二段**: Cloudflare tunnel → cloudflared → **Traefik(Ingress) Service** → app Service。
  cloudflared の tunnel ingress は各ホスト(`api.qkjudge.kisen.one` 等)を Traefik の Service(ClusterIP)へ向け、
  ホスト別の振り分けは Traefik Ingress 側で行う。Cloudflare(kisen.one) DNS に CNAME `*.cfargotunnel.com` を追加 (ユーザー操作、手順化)。
- **backup**: `mysqldump` CronJob → PVC (将来 R2/B2 等のオフサイトに拡張)。リストア手順も README に。
- staging は prod の overlay 差分のみ (別 namespace / 別 DB / dev image tag)。

## チェックリスト

### prod 立ち上げ
- [ ] leafeon に k3s 導入 (`nixos-config-private` 経由で宣言的に。鯖機直接設定はしない)
- [ ] `deploy/k3s/base` + `overlays/prod` (MariaDB StatefulSet+PVC+init、app Deployment/Service、Secret 参照、problems volume)
- [ ] Secret 投入手順を `deploy/k3s/README.md` 化 (平文非コミット)
- [ ] Ingress + cloudflared(Deployment/ConfigMap/Secret) + Cloudflare DNS 手順
- [ ] `api.qkjudge.kisen.one/ping` が外部から 200、主要フロー疎通
### staging
- [ ] `overlays/staging` (dev namespace / 別 DB / `dev.*` ホスト) を追加し疎通
### backup
- [ ] `mysqldump` CronJob + リストア手順
### 整合
- [ ] problems が起動時 clone + `POST /fetch/problems` で投入できる (webhook secret は Secret 経由)

## 完了条件

- [ ] prod / staging が k3s 上で稼働し、それぞれ外部 URL で主要フローが動く
- [ ] cloudflared がクラスタ内で動く。鯖機の直接設定変更はなく、ホスト変更 (k3s 等) は nixos-config-private 経由
- [ ] DB バックアップが定期取得される
- [ ] secret がリポジトリに含まれていない

## 作業ログ

- 2026-05-31: タスク生成。
