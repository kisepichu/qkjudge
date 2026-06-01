# deploy/k3s — leafeon k3s デプロイ (prod / staging)

旧 NeoShowcase のブランチ別デプロイ (master=prod / dev=staging) を leafeon の k3s 上で再現する
kustomize マニフェスト。詳細な背景は [`docs/MIGRATION.md`](../../docs/MIGRATION.md) を参照。

## 構成

```
Cloudflare tunnel ──► cloudflared (Deployment, ns: cloudflared)
                          │  ingress 設定は ConfigMap で管理
                          ▼
                      Traefik (k3s 同梱, ns: kube-system, web:80)
                          │  host で振り分け (Ingress)
            ┌─────────────┴─────────────┐
            ▼                           ▼
  api.qkjudge.kisen.one        api.dev.qkjudge.kisen.one
  ns: qkjudge-prod             ns: qkjudge-staging
    qkjudge-app (Deploy)         qkjudge-app (Deploy)
    qkjudge-mariadb (STS+PVC)    qkjudge-mariadb (STS+PVC)
    backup CronJob               backup CronJob
```

- frontend (`qkjudge.kisen.one`) は GitHub Pages 配信で **k3s 外**。tunnel には含めない。
- TLS 終端は Cloudflare edge。クラスタ内は平文 http (Traefik `web` entrypoint)。
- prod / staging は単一 tunnel・単一 Traefik を共有し、namespace で分離する。

```
deploy/k3s/
  base/                 # 環境共通 (MariaDB STS / app Deploy+Svc / Ingress / backup CronJob / 共通 env)
  overlays/prod/        # namespace=qkjudge-prod, image :master, host api.qkjudge.kisen.one
  overlays/staging/     # namespace=qkjudge-staging, image :dev,  host api.dev.qkjudge.kisen.one
  cloudflared/          # クラスタ singleton tunnel (両環境共有)
```

## 前提

- leafeon に k3s が導入済みであること。**k3s 導入はホスト変更なので鯖機を直接いじらず
  `nixos-config-private` 経由 (`services.k3s` 等) で宣言的に**行う (鯖機では pull + 反映のみ)。
- k3s 同梱の Traefik (Ingress) と local-path (StorageClass) を利用する (追加導入不要)。
- image は GHCR (`ghcr.io/kisepichu/qkjudge:{master,dev}`)。push は TASK-004 (CI/CD) で行う。
  本タスク時点では tag が存在しないため、手元で `docker build` したイメージを
  `k3s ctr images import` する等で暫定的に投入してもよい。
- `kubectl` が leafeon の k3s に対して設定済みであること。

## 投入が必要な secret / 設定 (リポジトリにコミットしない)

平文をコミットしないため、以下は namespace ごとに手動投入する (将来 Sealed Secrets / sops へ移行余地)。
コマンドはすべて **repo ルート**から実行する。

### 1. namespace を先に作る

Secret / ConfigMap は namespace が無いと作れないので先に作る。既存でも失敗しないよう冪等な
`--dry-run=client | apply` 形式にする (kustomize 適用時の Namespace とも冪等)。

```sh
kubectl create namespace qkjudge-prod --dry-run=client -o yaml | kubectl apply -f -
kubectl create namespace qkjudge-staging --dry-run=client -o yaml | kubectl apply -f -
```

### 2. アプリ secret (`qkjudge-secrets`)

各 namespace に投入する。`SESSION_KEY` は 64 hex (32 byte)。

```sh
kubectl -n qkjudge-prod create secret generic qkjudge-secrets \
  --from-literal=MARIADB_ROOT_PASSWORD='<root-pw>' \
  --from-literal=MARIADB_PASSWORD='<app-db-pw>' \
  --from-literal=SESSION_KEY="$(openssl rand -hex 32)" \
  --from-literal=GITHUB_WEBHOOK_TOKEN='<webhook-token>' \
  --from-literal=COMPILER_API_CLIENT_ID='<jdoodle-id>' \
  --from-literal=COMPILER_API_CLIENT_SECRET='<jdoodle-secret>'
```

staging も同様 (`-n qkjudge-staging`、値は別に発番推奨)。

> キー名は `src/main.rs` / MariaDB イメージが読む env 名と一致させること
> (`MARIADB_PASSWORD` はアプリと DB で共有、`MARIADB_ROOT_PASSWORD` は DB と backup CronJob が使う)。

### 3. 初期スキーマ ConfigMap (`mariadb-initdb`)

`migrations/v1.2.3.sql` (全テーブルの CREATE) を単一ソースに保つため、コミット済みファイルから
namespace ごとに作る。MariaDB は **PVC が空のとき (初回起動) だけ** これを実行する。

```sh
kubectl -n qkjudge-prod create configmap mariadb-initdb \
  --from-file=01-schema.sql=migrations/v1.2.3.sql
kubectl -n qkjudge-staging create configmap mariadb-initdb \
  --from-file=01-schema.sql=migrations/v1.2.3.sql
```

### 4. cloudflared tunnel (クラスタ singleton)

```sh
cloudflared tunnel login                       # ブラウザで kisen.one ゾーンを認可
cloudflared tunnel create qkjudge              # → ~/.cloudflared/<TUNNEL_ID>.json を出力
TUNNEL_ID=<上で表示された UUID>

kubectl create namespace cloudflared
kubectl -n cloudflared create secret generic cloudflared-credentials \
  --from-file=credentials.json="$HOME/.cloudflared/${TUNNEL_ID}.json"
```

`cloudflared/config.yaml` の `tunnel: REPLACE_WITH_TUNNEL_ID` を上記 `TUNNEL_ID` に**書き換えてから**
`cloudflared` を apply する (placeholder のままだと apply 自体は通るが、cloudflared が実行時に起動失敗する)。
UUID は secret ではないので、tunnel 作成後に確定値をコミットして固定するのを推奨 (環境ごとに一意)。
tunnel 発行〜この書き換え〜apply は live follow-up (本 PR のスコープ外) で行う。

## デプロイ

```sh
# secret / initdb ConfigMap 投入後に適用する。
kubectl apply -k deploy/k3s/overlays/prod
kubectl apply -k deploy/k3s/overlays/staging
kubectl apply -k deploy/k3s/cloudflared
```

ロールアウト確認:

```sh
kubectl -n qkjudge-prod rollout status deploy/qkjudge-app
kubectl -n qkjudge-prod rollout status statefulset/qkjudge-mariadb
kubectl -n cloudflared rollout status deploy/cloudflared
```

## DNS (Cloudflare CNAME)

tunnel をホスト名に紐付ける。`route dns` が Cloudflare DNS に `CNAME <host> <TUNNEL_ID>.cfargotunnel.com`
(proxied) を自動作成する。

```sh
cloudflared tunnel route dns qkjudge api.qkjudge.kisen.one
cloudflared tunnel route dns qkjudge api.dev.qkjudge.kisen.one
```

ダッシュボードで手動作成する場合は、`kisen.one` ゾーンに以下を **Proxied** で追加:

| Type  | Name             | Target                          |
| ----- | ---------------- | ------------------------------- |
| CNAME | api.qkjudge      | `<TUNNEL_ID>.cfargotunnel.com`  |
| CNAME | api.dev.qkjudge  | `<TUNNEL_ID>.cfargotunnel.com`  |

## 疎通確認

```sh
curl -i https://api.qkjudge.kisen.one/ping        # → 200
curl -i https://api.dev.qkjudge.kisen.one/ping    # → 200
```

主要フロー (signup/login/problems/submit→judge/submissions) を確認する。problems は
起動時に dist が clone される。webhook 経由の更新は `qkjudge-secrets` の `GITHUB_WEBHOOK_TOKEN`
で署名検証される (`POST /fetch/problems`)。

## バックアップ / リストア

`qkjudge-mariadb-backup` CronJob が毎日 18:00 UTC に `mariadb-dump` を取り、PVC `qkjudge-backup`
(`/backup/qkjudge-<ts>.sql.gz`) に保存する (14 日より古いものは削除)。手動実行・確認:

```sh
# 即時バックアップ
kubectl -n qkjudge-prod create job --from=cronjob/qkjudge-mariadb-backup backup-now
# 保存物の確認 (backup PVC をマウントした一時 pod)
kubectl -n qkjudge-prod run dump-ls --rm -it --image=busybox --restart=Never \
  --overrides='{"spec":{"containers":[{"name":"dump-ls","image":"busybox","command":["ls","-lh","/backup"],"volumeMounts":[{"name":"b","mountPath":"/backup"}]}],"volumes":[{"name":"b","persistentVolumeClaim":{"claimName":"qkjudge-backup"}}]}}'
```

リストア (`<dump>` を上の `dump-ls` で選び、backup PVC 上の `/backup/` から DB Service へ流し込む)。
backup PVC をマウントした一時 pod を立て、root パスワードは secret から参照する:

```sh
kubectl -n qkjudge-prod run mariadb-restore --rm -it --image=mariadb:11.4 --restart=Never \
  --overrides='{"spec":{"containers":[{"name":"mariadb-restore","image":"mariadb:11.4","stdin":true,"tty":true,"command":["sh","-c","gunzip < /backup/<dump>.sql.gz | mariadb -h qkjudge-mariadb -u root -p\"$MARIADB_ROOT_PASSWORD\" \"$MARIADB_DATABASE\""],"env":[{"name":"MARIADB_ROOT_PASSWORD","valueFrom":{"secretKeyRef":{"name":"qkjudge-secrets","key":"MARIADB_ROOT_PASSWORD"}}},{"name":"MARIADB_DATABASE","valueFrom":{"configMapKeyRef":{"name":"qkjudge-config","key":"MARIADB_DATABASE"}}}],"volumeMounts":[{"name":"b","mountPath":"/backup"}]}],"volumes":[{"name":"b","persistentVolumeClaim":{"claimName":"qkjudge-backup"}}]}}'
```

> 注: `qkjudge-config` は kustomize で hash 付き名になる (`qkjudge-config-xxxx`)。実際の名前は
> `kubectl -n qkjudge-prod get configmap` で確認して置き換えること。実運用では dump をローカルへ
> 取り出し (`kubectl cp`)、検証してから流すのが安全。

## secret ローテーション

`qkjudge-secrets` を更新したら、env を読み直させるため app を再起動する:

```sh
kubectl -n qkjudge-prod create secret generic qkjudge-secrets ... --dry-run=client -o yaml \
  | kubectl -n qkjudge-prod apply -f -
kubectl -n qkjudge-prod rollout restart deploy/qkjudge-app
```

`SESSION_KEY` を変えると全ユーザーがログアウトされる点に注意。

## 将来

- Secret 管理: Sealed Secrets / sops-age へ移行し宣言的にコミット可能にする。
- デプロイ反映: セルフホスト Actions Runner からの `kubectl apply` (TASK-004)。GitOps (Argo/Flux) は将来。
- バックアップ: PVC ローカルから R2/B2 等オフサイトへ拡張。
