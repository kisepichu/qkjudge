# TASK-008: ブランチ可変な staging 環境 (image-tag swap 方式)

## 参照

- `.github/workflows/deploy.yml` (現行 dev → staging の自動デプロイ。concurrency 構造の前提)
- `deploy/k3s/overlays/staging/` および `deploy/k3s/base/` (staging Deployment マニフェスト、
  Deployment 名 `qkjudge-app` / コンテナ名 `qkjudge`)
- TASK-003 / TASK-004 (k3s と CI/CD の基盤)

## 概要

任意の feature ブランチを staging で動作確認できるようにする。新しいホスト名・Ingress を作らず、
**既存 staging namespace の Deployment の `image` tag を、指定 ref のビルド成果に差し替える**方式を採る。
新ドメインを増やさないので Universal SSL (`*.kisen.one` 1 階層) 制約に抵触しない。

## 設計メモ

- **トリガ**: `workflow_dispatch` で `ref` (branch / tag / sha) を入力。
- **Runner**: 既存の self-hosted (leafeon) runner。
- **手順**:
  1. `actions/checkout@v4` で指定 ref を取得
  2. `docker build` → GHCR push (タグは `branch-<sanitized_ref>-<sha>` のように衝突しない命名。
     `ref` に `/` を含むブランチ名 (`feature/foo` 等) は Docker tag 仕様で不正になるため、
     workflow 側で `/` → `-` 等に sanitize してから渡す前提)
  3. `kubectl -n qkjudge-staging set image deploy/qkjudge-app qkjudge=<GHCR_TAG>`
     (Deployment 名 `qkjudge-app` / コンテナ名 `qkjudge` は `deploy/k3s/base/app-deployment.yaml` に一致)
  4. `kubectl -n qkjudge-staging rollout status deploy/qkjudge-app --timeout=120s`
- **競合制御**: `concurrency.group: staging-deploy` を dev auto-deploy と共有 (`cancel-in-progress: false`)。
  → dev push と手動 deploy が直列化され、image が予期せず上書きされない。
  - **前提**: 現行 `.github/workflows/deploy.yml` は workflow-level で
    `concurrency: { group: deploy-${{ github.ref }}, cancel-in-progress: true }` を使っている。
    本 TASK で `staging-deploy` 共有グループを実現するには、いずれかが必要:
    (a) `deploy.yml` の concurrency を **job-level** (staging deploy job 限定) に移して
        `staging-deploy` グループに揃える、または
    (b) staging deploy 専用の workflow に切り出して両 workflow で `staging-deploy` を共有。
    また dev 側を `cancel-in-progress: true` のままにすると手動 deploy 中の dev push で
    cancel が走るので、共有グループでは `false` に揃える。
- **戻し方**: 同じ workflow を `ref: dev` で再 run すれば、dev の現行 tag に戻る。
- **同時稼働**: 1 ブランチのみ (個人運用想定で十分)。複数並列が必要になったら namespace 切り替え方式へ拡張する。

## 限界 / 注意

- **staging DB は 1 個**: スキーマ破壊的変更を含む branch を流すと、dev に戻したとき DB が不整合になる
  可能性がある (今回の修正のように DB 変更を伴わない PR にのみ気軽に使う)。
- **GHCR 容量**: ブランチごとに tag が増えるので、retention policy (例: 30 日 / 50 tag) を別途設定する。
- **secret**: JDoodle / DB / SESSION_KEY は staging の k8s Secret をそのまま使う (ref に関わらず共通)。

## チェックリスト

- [ ] `.github/workflows/staging-deploy-branch.yml` 追加 (`workflow_dispatch` / `ref` 入力 / self-hosted runner)
- [ ] `concurrency.group: staging-deploy` を新 workflow と既存 `deploy.yml` の dev job 両方に設定
- [ ] GHCR の retention policy を確認・設定 (個人 GitHub 設定 or org policy)
- [ ] README か `docs/MIGRATION.md` 末尾に運用手順を追記 (手動 run / 戻し方)
- [ ] 試しに何でもない branch (例: README 修正 branch) を流して staging で表示確認、`ref: dev` で復元できることを確認

## 完了条件

- [ ] Actions タブから任意の branch / sha を staging に手動 deploy できる
- [ ] dev auto-deploy と手動 deploy が直列に実行され競合しない
- [ ] `ref: dev` で dev の最新状態に戻せる

## 作業ログ

- 2026-06-17: タスク生成。TASK-007 の検証フローを楽にするために発案。
