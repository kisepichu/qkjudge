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
- [ ] `.github/workflows/` でビルド+GHCR push (`:{branch}`/`:{sha}`)、`cargo test/clippy/fmt`
### self-hosted runner
- [ ] leafeon にセルフホスト Runner を登録 (最小権限・隔離)
- [ ] deploy ワークフロー: master→prod / dev→staging を kustomize apply / set image
- [ ] フォーク PR で Runner/Secret が露出しないトリガ設計
### 検証
- [ ] dev に push → staging が自動更新されることを確認
- [ ] master に push → prod が自動更新されることを確認

## 完了条件

- [ ] ブランチへの push で対応環境が自動デプロイされる
- [ ] セルフホスト Runner がフォーク PR の untrusted 実行に晒されない
- [ ] Secret/kubeconfig のスコープが最小

## 作業ログ

- 2026-05-31: タスク生成。セキュリティ堅牢化を必須要件として明記。
