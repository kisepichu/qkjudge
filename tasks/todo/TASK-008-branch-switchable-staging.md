# TASK-008: ブランチ可変な staging 環境 (image-tag swap 方式)

## 参照

- `.github/workflows/deploy.yml` (現行 dev → staging の自動デプロイ。concurrency group を共有する)
- `k8s/staging/` (staging Deployment マニフェスト)
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
  3. `kubectl -n qkjudge-staging set image deploy/qkjudge qkjudge=<GHCR_TAG>`
  4. `kubectl -n qkjudge-staging rollout status deploy/qkjudge --timeout=120s`
- **競合制御**: `concurrency.group: staging-deploy` を dev auto-deploy と共有 (`cancel-in-progress: false`)。
  → dev push と手動 deploy が直列化され、image が予期せず上書きされない。
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
