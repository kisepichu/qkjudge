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

> **このタスク (PR) のスコープ** = `deploy/k3s/` マニフェスト + README の整備 (ユーザー確認済み)。
> k3s 導入・tunnel 発行・DNS・外部疎通確認は host/Cloudflare 側操作のため **live follow-up** とし、
> 該当チェックは未完のまま残す。problems volume は **emptyDir** (git から再現可能) を採用。

### prod 立ち上げ
- [ ] leafeon に k3s 導入 (`nixos-config-private` 経由で宣言的に。鯖機直接設定はしない) ← **host 側 follow-up**
- [x] `deploy/k3s/base` + `overlays/prod` (MariaDB StatefulSet+PVC+init、app Deployment/Service、Secret 参照、problems volume)
- [x] Secret 投入手順を `deploy/k3s/README.md` 化 (平文非コミット)
- [x] Ingress + cloudflared(Deployment/ConfigMap/Secret) + Cloudflare DNS 手順 (マニフェスト + README。DNS/tunnel 発行の実行は host/Cloudflare 側)
- [ ] `api.qkjudge.kisen.one/ping` が外部から 200、主要フロー疎通 ← **live cluster follow-up**
### staging
- [x] `overlays/staging` (dev namespace / `dev.*` ホスト) を追加 (疎通確認は live follow-up)
### backup
- [x] `mysqldump` CronJob + リストア手順 (`backup-cronjob.yaml` + README)
### 整合
- [x] problems が起動時 clone + `POST /fetch/problems` で投入できる (webhook secret は Secret 経由) ← entrypoint + emptyDir + `GITHUB_WEBHOOK_TOKEN` を Secret で配線

## 完了条件

- [ ] prod / staging が k3s 上で稼働し、それぞれ外部 URL で主要フローが動く ← **live follow-up** (マニフェストは build 検証済み)
- [ ] cloudflared がクラスタ内で動く。鯖機の直接設定変更はなく、ホスト変更 (k3s 等) は nixos-config-private 経由 ← マニフェスト準備済み、稼働確認は live follow-up
- [ ] DB バックアップが定期取得される ← CronJob 準備済み、稼働確認は live follow-up
- [x] secret がリポジトリに含まれていない (gitleaks staged scan: no leaks。secret/initdb/creds はすべて namespace へ手動投入)

## 作業ログ

- 2026-05-31: タスク生成。
- 2026-06-02: `deploy/k3s/` マニフェスト + README を整備。
  - **スコープ判断** (ユーザー確認): 本 PR = マニフェスト + README まで。k3s 導入 (nixos-config-private)・
    tunnel 発行・DNS・外部疎通は host/Cloudflare 側 follow-up。problems volume = **emptyDir**。
  - **構成**: kustomize `base/` (MariaDB StatefulSet+PVC / app Deployment+Service / Ingress / backup CronJob /
    共通 env configMapGenerator) + `overlays/{prod,staging}` (namespace / image tag master|dev /
    Ingress host / CORS origin の差分) + `cloudflared/` (クラスタ singleton tunnel、両環境共有)。
  - **MariaDB**: `mariadb:11.4` (TASK-002 と一致)、`volumeClaimTemplates` local-path 5Gi。初期スキーマは
    `migrations/v1.2.3.sql` を単一ソースに保つため ConfigMap `mariadb-initdb` を README 手順で namespace に
    手動作成 (kustomize の load-restrictor 回避)。PVC 空のとき (初回) のみ initdb.d 実行。
  - **app**: env は `qkjudge-config` (configMap, 非 secret) + `qkjudge-secrets` (手動投入 Secret) を envFrom。
    `MARIADB_USER`(DB)/`MARIADB_USERNAME`(app) の名前不一致は両キーを同値で持たせた。`/ping` を probe に使用。
  - **routing**: Cloudflare tunnel → cloudflared → Traefik(`web`:80, k3s 同梱) → Ingress(host 振り分け) → app。
    cloudflared は credentials-file モード (ingress を ConfigMap で宣言)。tunnel id は config.yaml の
    placeholder、credentials.json は Secret。TLS 終端は Cloudflare edge、クラスタ内は平文 http。
  - **secret 非コミット**: Secret/initdb ConfigMap/cloudflared creds はすべて固定名で参照し README の
    `kubectl create` 手順で投入 (secretGenerator/平文は使わない)。`gitleaks protect --staged` = no leaks。
  - **検証**: `kubectl kustomize` で prod/staging/cloudflared の 3 build が成功。configMap hash 参照
    (envFrom / configMapKeyRef / volumes) が全箇所で一致、image tag (master/dev)・namespace・host・
    CORS origin の overlay 差分、外部 Secret/initdb の固定名参照を render 出力で確認。
  - **未検証 (host 依存)**: 実 k3s での apply・外部 URL 疎通・cloudflared 稼働・backup 実行は live follow-up。
    cloudflared image tag は要更新確認 (`cloudflare/cloudflared` のリリースに追随)。
