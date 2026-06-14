# TASK-006: フロント切替 + judge.tqk.blue リダイレクト (Phase 4-5 / qkjudge-UI repo)

## 参照

- `docs/MIGRATION.md`「環境マッピング」「Cookie/CORS」「失われたもの」
- repo: `~/repos/qkjudge-UI` (origin `kisepichu/qkjudge-UI`、GitHub Pages デプロイ `.github/workflows/pages.yml`)
- API URL は `VITE_API_URL` (`.env` = prod, `.env.development` = local)
- DNS: `kisen.one`(Cloudflare)、`tqk.blue`(お名前.com) いずれも操作可
- TASK-005 (legacy 配信エンドポイント)

## 概要

フロントの API 接続先を新サーバーに切替え、新ドメイン `qkjudge.kisen.one` で公開する。
トップに再登録の告知を出し、提出一覧で legacy を表示する。旧 `judge.tqk.blue` は新ドメインへ
リダイレクトする。**実装は qkjudge-UI リポジトリで PR。**

## 設計メモ

- **API URL**: prod `.env` → `https://qkjudge-api.kisen.one`、staging `.env` → `https://qkjudge-api-stg.kisen.one`。
- **Cookie**: フロント `qkjudge.kisen.one` と API `qkjudge-api.kisen.one` は **same-site**(同一 registrable domain `kisen.one`)
  なので `SameSite=Lax; Secure` + `withCredentials` で問題なく動く。Cookie は **API ホスト上で host-only に完結**し
  (Domain 属性で親に広げない)、**HTTPS / `Secure` 前提**。これらが満たされることを確認する。
- **ドメイン/公開**: GitHub Pages の独自ドメインを `qkjudge.kisen.one` に (CNAME ファイル + Cloudflare DNS)。
  staging フロントは `dev.qkjudge.kisen.one` (別 Pages か k3s 配信かは TASK-003/004 と整合)。
- **再登録告知**: トップページに「旧 judge.tqk.blue のアカウントは移行されていません。再登録してください」を表示。
- **legacy 表示**: 提出一覧で新提出が尽きた後に TASK-005 の `/legacy/*` を表示、author に `[legacy]` prefix。
  旧サーバー CORS は `judge.tqk.blue` 限定のため**ブラウザ直叩きはしない** (必ず新サーバーの legacy
  エンドポイント経由)。
- **judge.tqk.blue リダイレクト**: 旧 Pages のビルドを、`qkjudge.kisen.one` への
  リダイレクト + 告知ページに差し替える (meta refresh / JS)。DNS は据置でも内容差し替えで実現可。

## チェックリスト

### API 切替
- [ ] `.env`/`.env.development` の `VITE_API_URL` を新 API に更新
- [ ] ログイン/whoami/submit/submissions が新 API + `SameSite=Lax` cookie で動く
### ドメイン公開
- [ ] CNAME + Cloudflare DNS で `qkjudge.kisen.one` 公開、(可能なら) `dev.` も
### UI 変更
- [ ] トップに再登録告知
- [ ] 提出一覧末尾に legacy 表示 (`[legacy]` prefix、TASK-005 と整合)
### リダイレクト
- [ ] `judge.tqk.blue` を新ドメインへリダイレクト + 告知に差し替え

## 完了条件

- [ ] `qkjudge.kisen.one` が新サーバーに繋がり主要フローが動く
- [ ] `judge.tqk.blue` 訪問者が新ドメインへ誘導され、再登録告知を見られる
- [ ] legacy 提出が一覧で確認できる

## 作業ログ

- 2026-05-31: タスク生成。
