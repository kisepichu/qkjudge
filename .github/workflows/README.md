# CI/CD (TASK-004)

旧 NeoShowcase の「push したら自動反映」を GitHub Actions + leafeon のセルフホスト Runner で再現する。
背景は [`docs/MIGRATION.md`](../../docs/MIGRATION.md)、k3s 側は [`deploy/k3s/README.md`](../../deploy/k3s/README.md)。

## ワークフロー

| file         | trigger                          | runner         | 役割                                                              |
| ------------ | -------------------------------- | -------------- | ----------------------------------------------------------------- |
| `ci.yml`     | `pull_request` / push master,dev | GitHub-hosted  | `cargo fmt --check` / `clippy -D warnings` / `cargo test`。secret 不使用 |
| `deploy.yml` | push master,dev のみ             | build=hosted, deploy=self-hosted | build→GHCR push (`:{branch}`/`:{sha}`)、deploy=対象 ns の image 差し替え |

- `master` → prod (`qkjudge-prod`, `api.qkjudge.kisen.one`)、`dev` → staging (`qkjudge-staging`, `api.dev.qkjudge.kisen.one`)。
- deploy は GitHub-hosted で GHCR に push 後、self-hosted Runner が `kubectl set image` で
  イミュータブルな `:{sha}` タグへ差し替え `rollout status` で完了を待つ。

## セキュリティ堅牢化 (TASK-004 必須要件)

1. **fork PR を Runner に載せない**: `deploy.yml` のトリガは `push: [master, dev]` のみ。
   fork からの `pull_request` では起動しないので、untrusted コードがセルフホスト Runner で
   実行されず、secret も渡らない。CI (`ci.yml`) は PR でも回るが GitHub-hosted・secret 不使用。
2. **kubeconfig は namespace-scoped SA トークン**: cluster-admin は Runner に置かない。
   `deploy/k3s/base/deployer-rbac.yaml` の ServiceAccount `qkjudge-deployer` は当該 namespace の
   Deployment 更新権限のみ (set image + rollout 監視)。インフラ apply 権限は持たせない。
3. **prod / staging を Environment で分離**: GitHub Environment (`prod` / `staging`) ごとに
   `KUBECONFIG_DATA` を分けて注入する。ブランチ→Environment は `deploy.yml` が解決し、
   staging ジョブには staging namespace のトークンしか渡らない (prod を触れない)。
4. **Runner ラベル固定**: deploy ジョブは `runs-on: [self-hosted, leafeon]`。build は GitHub-hosted。

## セットアップ (live follow-up — 本 PR スコープ外)

実 leafeon が必要なので、以下は host 側 follow-up で行う。

### 1. デプロイ用 RBAC を apply (namespace 作成・secret 投入後)

```sh
kubectl apply -k deploy/k3s/overlays/prod      # qkjudge-deployer SA/Role/RoleBinding/token を含む
kubectl apply -k deploy/k3s/overlays/staging
```

### 2. SA トークンから Runner 用 kubeconfig を組み立てる (namespace ごと)

`qkjudge-deployer-token` Secret から token と CA を取り出し、k3s API server URL を入れる。
**prod / staging で別々に作り、それぞれの Environment secret に入れる**。

```sh
NS=qkjudge-prod                                  # staging は qkjudge-staging
SERVER=https://<leafeon-k3s-api>:6443            # Runner から到達できる k3s API endpoint
TOKEN=$(kubectl -n "$NS" get secret qkjudge-deployer-token -o jsonpath='{.data.token}' | base64 -d)
CA=$(kubectl -n "$NS" get secret qkjudge-deployer-token -o jsonpath='{.data.ca\.crt}')

cat > /tmp/kubeconfig-$NS <<EOF
apiVersion: v1
kind: Config
clusters:
  - name: leafeon
    cluster:
      server: $SERVER
      certificate-authority-data: $CA
users:
  - name: qkjudge-deployer
    user:
      token: $TOKEN
contexts:
  - name: deployer
    context:
      cluster: leafeon
      user: qkjudge-deployer
      namespace: $NS
current-context: deployer
EOF

# 権限確認 (deployment 更新は yes、infra は no になるはず)
KUBECONFIG=/tmp/kubeconfig-$NS kubectl auth can-i patch deployments -n "$NS"   # → yes
KUBECONFIG=/tmp/kubeconfig-$NS kubectl auth can-i patch deployments -n qkjudge-staging  # prod 用 → no

base64 -w0 /tmp/kubeconfig-$NS    # ← この値を Environment secret KUBECONFIG_DATA に登録
rm -f /tmp/kubeconfig-$NS
```

### 3. GitHub Environment と secret

リポジトリ Settings → Environments で `prod` / `staging` を作成し、それぞれに secret
`KUBECONFIG_DATA` (上の base64) を登録する。prod には必要に応じて protection rule
(required reviewers 等) を付ける。Runner からは `secrets.KUBECONFIG_DATA` で参照される。

### 4. セルフホスト Runner を leafeon に登録 (最小権限・隔離)

- 専用の最小権限ユーザー (sudo 不可) で runner サービスを動かす。ホスト変更は
  `nixos-config-private` 経由で宣言的に (鯖機を直接いじらない方針)。
- ラベル `leafeon` を付与 (`deploy.yml` の `runs-on` と一致)。
- Runner からは k3s API への到達と `kubectl` のみが必要。Docker/ホストの広い権限は与えない。
- リポジトリ Settings → Actions → Runners から登録トークンを取得して接続。

### 5. 疎通確認 (live)

```sh
git commit --allow-empty -m "ci: trigger staging deploy"; git push origin dev
# → Actions の deploy が staging を更新。kubectl -n qkjudge-staging rollout status deploy/qkjudge-app で確認。
# master でも同様に prod を確認する。
```

## 将来

- GitOps (Argo CD / Flux) へ移行し、Runner からの命令的 apply をやめる (`docs/MIGRATION.md` 決定事項)。
- イメージ参照を tag (`:{sha}`) から digest 固定にし、署名 (cosign) 検証を足す。
