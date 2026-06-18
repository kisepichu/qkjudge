# TASK-006: フロント切替 + judge.tqk.blue リダイレクト (Phase 4-5 / qkjudge-UI repo)

## 参照

- `docs/MIGRATION.md`「環境マッピング」「Cookie/CORS」「失われたもの」
- repo: `~/repos/qkjudge-UI` (origin `kisepichu/qkjudge-UI`、Cloudflare Pages 配信)
- API URL は `VITE_API_URL` (`.env` = prod, `.env.development` = local 開発用、vite proxy 経由)
- DNS: `kisen.one`(Cloudflare)、`tqk.blue`(お名前.com) いずれも操作可
- TASK-005 (legacy 配信エンドポイント)

## 概要

フロントの API 接続先を新サーバーに切替え、新ドメイン `qkjudge.kisen.one` で公開する。
トップに再登録の告知を出し、提出一覧で legacy を表示する。旧 `judge.tqk.blue` は新ドメインへ
リダイレクトする。**実装は qkjudge-UI リポジトリで PR。**

## 設計メモ

- **API URL**: prod `.env` → `https://qkjudge-api.kisen.one`、Cloudflare Pages の Preview env で
  `VITE_API_URL=https://qkjudge-api-stg.kisen.one` を設定 (key と value を分けて入力する点に注意。
  value に `VITE_API_URL=` プレフィクスを付けると bundle 内で `const a = "VITE_API_URL=https://..."`
  となりリクエスト URL が壊れる事故が発生したため、出し直し時はここを確認)。
- **Cookie**: フロント `qkjudge.kisen.one` と API `qkjudge-api.kisen.one` は **same-site**(同一 registrable domain `kisen.one`)
  なので `SameSite=Lax; Secure` + `withCredentials` で問題なく動く。Cookie は **API ホスト上で host-only に完結**し
  (Domain 属性で親に広げない)、**HTTPS / `Secure` 前提**。
- **ドメイン/公開**: **Cloudflare Pages** に移行 (旧 GitHub Pages から)。
  - Production branch `master` → `qkjudge.kisen.one`
  - Preview branch `dev` → `qkjudge-stg.kisen.one` (branch alias)
  - DNS は両方 Cloudflare 側で **proxied CNAME** `<project>.pages.dev` / `dev.<project>.pages.dev`
  - staging を 1 階層 (`qkjudge-stg.kisen.one`) にしたのは Universal SSL が 1 階層のみ
    カバーするため (`dev.qkjudge.kisen.one` のような 2 階層は不可)。API 側の
    `qkjudge-api-stg.kisen.one` と命名を揃えた。
- **PR preview の CORS**: 静的単一 origin 設計だと PR preview URL (`<hash>.qkjudge-ui.pages.dev`)
  からの fetch が CORS で蹴られて動作確認できなかったので、staging 側だけ
  `CORS_ALLOW_ORIGIN=https://qkjudge-stg.kisen.one,https://*.qkjudge-ui.pages.dev` の CSV +
  wildcard suffix 形式に拡張した (Issue #33 / PR #34、actix-cors `allowed_origin_fn` 採用)。
  prod は `https://qkjudge.kisen.one` 単一のまま。
- **再登録告知**: トップページに「旧 judge.tqk.blue のアカウントは移行されていません。
  再登録してください」をヒーロー画像直下に表示。
- **legacy 表示**: 新側 pages_number の最終ページで legacy 先頭を埋めて 10 件にし、
  以降は legacy 全体を shift したオフセットで連続ページングする。legacy 全件 (58 件)
  は read-only スナップショットなので mount 時に並列 fetch して memory に持ち、
  以降のページ移動は memory slice のみ。author / id は `[legacy] <name>` / `#L-<id>` で
  prefix を付けて衝突回避。
- **judge.tqk.blue リダイレクト**: kisepichu/qkjudge-UI の **orphan branch `legacy-redirect`**
  を作って `index.html` + `404.html` (任意 path で SPA fallback) + `CNAME` だけを置き、
  GitHub Pages の source をこの branch + `build_type=legacy` に切り替えて配信。
  DNS は据置 (judge.tqk.blue → tqkoh.github.io、Pages が repo claim でルーティング)。
  JS 即時遷移 + meta refresh 3 秒 fallback + 告知文 (no-JS でも見える)。

## チェックリスト

### API 切替
- [x] `.env` の `VITE_API_URL` を `https://qkjudge-api.kisen.one` に更新、`.env.development`
      は vite proxy 経由のままで target だけ staging API に向ける
- [x] ログイン/whoami/submit/submissions が新 API + `SameSite=Lax` cookie で動く
### ドメイン公開
- [x] Cloudflare Pages 採用 (GitHub Pages 廃止、`dist/` のリポ commit も止めて
      `public/_redirects` で SPA fallback)、 production `qkjudge.kisen.one` /
      preview alias `qkjudge-stg.kisen.one` を proxied CNAME で公開
### UI 変更
- [x] Home に再登録告知バナー (ヒーロー直下 / WIP カードの上)
- [x] 提出一覧で legacy を連続ページング (新側末尾を埋める + shift)、`[legacy]` prefix と
      `#L-<id>` 表示、`/legacy/submissions/:id` 詳細ルートも対応
### リダイレクト
- [x] `judge.tqk.blue` を `legacy-redirect` branch (orphan) の Pages 配信で新ドメインへ
      リダイレクト + 告知 fallback

## 完了条件

- [x] `qkjudge.kisen.one` が新サーバーに繋がり主要フローが動く
- [x] `judge.tqk.blue` 訪問者が新ドメインへ誘導され、再登録告知を見られる
- [x] legacy 提出が一覧で確認できる

## 作業ログ

- 2026-05-31: タスク生成。
- 2026-06-18 Phase A: staging frontend host を `qkjudge-stg.kisen.one` に確定
  (Universal SSL の 1 階層制約のため `dev.qkjudge.kisen.one` は使わず、
  `qkjudge-api-stg.kisen.one` と命名を揃える)。`deploy/k3s/overlays/staging/kustomization.yaml`
  の `CORS_ALLOW_ORIGIN` を新ホストに更新 (PR #31)。configMap は CD で apply されない
  ため leafeon 上で手動 `kubectl apply -k overlays/staging` + `rollout restart` が必要だった
  (Issue #32 として追跡)。
- 2026-06-18 Phase B: qkjudge-UI 側を Cloudflare Pages に移行。GitHub Pages workflow と
  CNAME・コミット済み `dist/` を削除、`public/_redirects` を追加 (PR #8)。提出一覧に legacy
  末尾連結 + `/legacy/submissions/:id` ルート + Home に再登録告知も同 PR に同梱。
  Copilot review で SPA navigation 系の race / useEffect 依存抜けを 8 round に渡って指摘され、
  AbortController + `setTimeout` cleanup を全 fetch に展開する形で対応。
- 2026-06-18 Phase B (続き): PR merge 後の staging が 404 を返したため原因調査 →
  Cloudflare Pages の env vars value 欄に `VITE_API_URL=https://...` (key 接頭辞ごと)
  を貼っていたことが判明、bundle 内で `const a = "VITE_API_URL=https://..."` になり
  リクエスト URL が壊れていた。env を直して再 deploy + Problems.tsx の `setLoading(true)`
  typo (元コード) + `res.data?.foo ?? []` の defensive guard を PR #9 で投入し、復旧。
  ついでに Home 告知バナーをヒーロー直下に移動。
- 2026-06-18 Phase B (続き): PR preview URL (`<hash>.qkjudge-ui.pages.dev`) からの fetch が
  staging API CORS に蹴られて preview デバッグできない問題が判明 → 別 issue (#33) として
  qkjudge メインで `CORS_ALLOW_ORIGIN` を CSV + wildcard suffix 対応に拡張し
  `https://*.qkjudge-ui.pages.dev` を staging だけ許可 (PR #34)。これで PR ブランチごとの
  preview URL でも staging API を叩ける。
- 2026-06-18 Phase B (続き): UI 微調整として「新側 pages_number 最終ページに legacy 先頭で
  埋めて 10 件にし、以降は legacy をシフトしてページング」と「legacy ID 表示を `#L-<id>` に」
  を PR #10 で実装。Copilot review で `Math.max(0, ...)` ガード / `Promise.all` per-page
  catch / `<Pagination page>` clamp の 3 round対応。
- 2026-06-18 Phase D: qkjudge-UI dev→master (PR #11) と qkjudge dev→master (PR #35) を merge
  し、Cloudflare Pages Production deploy + prod k8s rollout で `qkjudge.kisen.one` を新 UI
  に切替。prod overlay は configMap 変更なしのため image set 反映のみで完結。
- 2026-06-18 Phase C: kisepichu/qkjudge-UI に orphan branch `legacy-redirect` を作成し
  `index.html` (JS 即時 + meta refresh 3 秒 fallback + 告知)、`404.html` (同 redirect)、
  `CNAME=judge.tqk.blue` を配置。`gh api PUT /repos/.../pages` で source を
  `legacy-redirect` + `build_type=legacy` に切替、初回は build が自動で trigger されないため
  `gh api POST .../pages/builds` で手動キック。`judge.tqk.blue` から `qkjudge.kisen.one`
  への遷移を確認、TASK-006 全体クローズ。
