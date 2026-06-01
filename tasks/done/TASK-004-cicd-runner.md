# TASK-004: CI/CD (GHCR + セルフホスト Runner) (Phase 3)

## 参照

- `docs/MIGRATION.md`「デプロイ反映」
- TASK-002 (sqlx offline ビルド) / TASK-003 (k3s マニフェスト・namespace)

## 概要

「push したら自動反映」を再現する。GitHub Actions でイメージをビルドして GHCR に push し、
leafeon 上のセルフホスト Runner が k3s へ反映する (`master`→prod / `dev`→staging)。
セルフホスト Runner は攻撃面なので**慎重に堅牢化**する。

## 設計メモ

- **build (GitHub-hosted runner)**: push 時に `SQLX_OFFLINE=true` でマルチステージビルド →
  `ghcr.io/kisepichu/qkjudge` に `:{branch}` と `:{sha}` で push。`cargo test`/`clippy`/`fmt` を CI チェックに。
- **deploy (self-hosted runner on leafeon)**: ビルド成功後、ローカル `kubectl set image`(または kustomize apply)
  で該当 namespace を更新。k3s API を外部公開しないための構成。
- **セルフホスト Runner 堅牢化 (必須)**:
  - 専用最小権限ユーザー / コンテナ内で実行、ホスト権限を絞る。
  - **フォーク PR では実行しない** (`pull_request` の untrusted コードを Runner に載せない。deploy は
    `push` to master/dev のみをトリガに)。
  - **prod/staging を RBAC で分離**: cluster-admin kubeconfig を Runner に持たせず、
    **namespace-scoped な ServiceAccount トークン + Role/RoleBinding**(各 namespace の Deployment 更新権限のみ)を使う。
    同一 Runner が両環境を扱う場合でも、staging デプロイ経路から prod を触れないようにする
    (ジョブごとに対象 namespace の SA トークンだけを渡す)。
  - Runner ラベルでデプロイジョブだけを self-hosted に固定 (ビルドは GitHub-hosted)。
- GitOps(Argo/Flux) への将来移行を見据え、deploy は kustomize overlay 適用に寄せておく。

## チェックリスト

### build CI
- [x] `.github/workflows/` でビルド+GHCR push (`:{branch}`/`:{sha}`)、`cargo test/clippy/fmt`
      ← `ci.yml` (fmt/clippy -D warnings/test) + `deploy.yml` の build ジョブ (GHCR push)
### self-hosted runner
- [ ] leafeon にセルフホスト Runner を登録 (最小権限・隔離) ← 登録手順を docs 化済み、登録自体は live follow-up
- [x] deploy ワークフロー: master→prod / dev→staging を kustomize apply / set image
      ← `deploy.yml` (self-hosted, `kubectl set image` で `:{sha}` 固定 + rollout)
- [x] フォーク PR で Runner/Secret が露出しないトリガ設計 ← deploy は `push: [master,dev]` のみ
### 検証
- [ ] dev に push → staging が自動更新されることを確認 ← live follow-up (実 Runner/k3s 必要)
- [ ] master に push → prod が自動更新されることを確認 ← live follow-up

## 完了条件

- [ ] ブランチへの push で対応環境が自動デプロイされる ← ワークフロー/RBAC 準備済み、稼働確認は live follow-up
- [x] セルフホスト Runner がフォーク PR の untrusted 実行に晒されない ← push-only トリガ、CI は hosted・secret 不使用
- [x] Secret/kubeconfig のスコープが最小 ← namespace-scoped SA (`qkjudge-deployer`) + Environment 分離

## 作業ログ

- 2026-05-31: タスク生成。セキュリティ堅牢化を必須要件として明記。
- 2026-06-02: CI/CD ワークフロー + デプロイ RBAC + docs を整備。
  - **スコープ判断** (ユーザー確認): 本 PR = ワークフロー + RBAC マニフェスト + docs まで。Runner 登録・
    実 push でのデプロイ検証は leafeon ホスト依存のため live follow-up (TASK-003 と同方針)。
  - **`ci.yml`** (GitHub-hosted, secret 不使用): `cargo fmt --check` / `clippy -D warnings` / `cargo test`。
    `pull_request` (fork 含む) でも安全に回せる。toolchain は Dockerfile builder と同じ 1.96.0 に固定。
  - **clippy `-D warnings` 有効化** (prek の TODO 消化): 既存 lint 約60件 (useless_vec / redundant_field_names /
    unused_parens / dead_code 等) を解消。機械的修正は subagent に offload (`CLAUDE_OFFLOAD_CONFIG_DIR`
    未設定のため Agent ツールにフォールバック)。`cargo clippy --fix` で大半、残り手動。**裏取り**: 削除は
    「never constructed」な未使用 struct/variant のみ (get_execute の 3 struct、各所の重複 `Problem`、
    `SolutionStatusNum`、未使用 HMAC type alias)。`query_as!` のデシリアライズ対象 (`ProblemLocation` 等) と
    267 件の言語メタテーブル `Language` は削除せず `#[allow(dead_code)]` + 理由コメント。HMAC テストの
    バイト列は不変。`fmt`/`clippy -D warnings`/`test` (2 passed) を再実行して確認。`.pre-commit-config.yaml`
    の clippy pre-push フックも再有効化。
  - **`deploy.yml`**: build (GitHub-hosted) → GHCR push (`:{branch}` 可変タグ + `:{sha}` イミュータブル)、
    deploy (self-hosted `[self-hosted, leafeon]`) で対象 namespace の `qkjudge-app` image を `:{sha}` に
    `set image` し `rollout status` 監視。infra apply は手動運用のまま (権限最小化)。
  - **セキュリティ堅牢化**: (1) deploy は `push: [master,dev]` のみ → fork PR の untrusted コードを Runner に
    載せない。(2) cluster-admin を Runner に渡さず namespace-scoped SA `qkjudge-deployer` (Deployment 更新権限
    のみ。`deploy/k3s/base/deployer-rbac.yaml`)。(3) GitHub Environment (`prod`/`staging`) で `KUBECONFIG_DATA`
    を分離注入し、ブランチ→Environment 解決で staging 経路から prod を触れない。(4) deploy ジョブを
    self-hosted ラベルに固定、build は hosted。kubeconfig は secret を env 経由で渡し、ジョブ後に削除。
  - **検証**: `cargo fmt/clippy -D warnings/test` 緑。`kubectl kustomize overlays/{prod,staging}` で deployer
    SA/Role/RoleBinding/token-Secret が各 namespace に正しく render。`actionlint` = exit 0
    (`.github/actionlint.yaml` で `leafeon` ラベル宣言)。YAML 構文 OK。
  - **未検証 (host 依存)**: 実 Runner 登録・SA トークン kubeconfig 発行・Environment secret 登録・実 push での
    prod/staging 自動反映は live follow-up (手順は `.github/workflows/README.md` に記載)。
