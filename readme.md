# QK Judge

休憩ジャッジ

競プロ問題公開したい！

コンパイルや実行部分はいったん外部アプリに任せる

サイト(WIP): [QK Judge](https://judge.tqk.blue)

サーバー(:koko:): [qkjudge](https://github.com/tqkoh/qkjudge)<br>
クライアント: [qkjudge-UI](https://github.com/tqkoh/qkjudge-UI)<br>
問題: [qkjudge-problems](https://github.com/tqkoh/qkjudge-problems)<br>

[API 予定](https://apis.tqk.blue/)

[開発メモ](https://github.com/tqkoh/qkjudge/blob/dev/memo.md)

## セキュリティと運用

- Rust 1.94 以上が必要。Rust 1.96 / MariaDB 10.6 で検証。ビルド時の SQL 検査には、スキーマを適用したローカル DB を指す `DATABASE_URL` が必要。
- 依存関係は `Cargo.lock` を使って `cargo build --locked` / `cargo test --locked` で固定し、`cargo audit` で定期的に確認する。2026-10-09 の更新後は RustSec の脆弱性・警告とも 0 件。これはアプリ全体の安全性の保証ではない。
- 外向き HTTPS はホストの TLS ライブラリにも依存するため、OS / OpenSSL / MariaDB / Git も更新する。HTTP/2 と TLS 終端はリバースプロキシ側で管理し、アプリの待受は HTTP のみ。
- `POST /execute` はログイン必須。`POST /problems` は既存の `admin` / `tqk` のみ。一般登録ではこの二つの名前（ASCII 大文字小文字違いを含む）を予約する。管理アカウントは公開登録ではなく、管理者が DB 側で作成する。既存の管理名を誰が登録したかはデプロイ前に確認する。
- パスワードは UTF-8 **72 バイト以下**、ユーザー名は30文字以下。旧版で72バイトを超えるパスワードを登録した利用者は、管理者によるパスワード再設定が必要。bcrypt の旧ハッシュから切り捨てた末尾は復元できない。
- ブラウザからの Origin は `https://judge.tqk.blue` のみ許可する。クロスサイト Cookie のため `SameSite=None; Secure; HttpOnly` は維持し、不一致 Origin は処理前に拒否する。ログアウトと `tqk` による手動取込には、この Origin ヘッダーが必要。
- GitHub の問題取込は `X-Hub-Signature-256: sha256=<64桁のhex>` と `GITHUB_WEBHOOK_TOKEN` で生の本文バイトを検証する。SHA-1 ヘッダーは不要。Webhook は Cookie / Origin に依存せず利用できる。
- `PROBLEMS_ROOT` は問題リポジトリのルートディレクトリ。問題パスは相対パスにし、root 外を指すパスやシンボリックリンク、通常ファイルでないテストデータを受け付けない。提出・再ジャッジには一組以上の入出力ファイルが必要で、欠けた出力ファイルはエラーになる。提出ソースは DB の `TEXT` に合わせて65,535バイト以下。
- 再ジャッジは言語・公開状態・テストデータを確認してから、結果とタスクをトランザクションでリセットする。ファイル読取失敗やコンパイラ通信失敗は終了結果 `UE` にする。コンパイラ HTTP 要求には30秒のタイムアウトを設ける。
- 提出ソース・保存済みテスト入出力は従来どおり公開。問題を非表示にしても過去の提出やタスクは非公開にならない。秘密の問題・テスト・ソースを置かない。
- 旧版はセッション Cookie、コンパイラ資格情報、Webhook の期待署名をログへ出していた。過去のログへのアクセスを確認し、信頼範囲外に見られた可能性がある場合はコンパイラ資格情報と Webhook トークンを交換する。サーバー再起動で旧セッションを無効化する。

残る制約：公開登録があるためログイン必須だけではコンパイラの予算枯渇を防げない。アカウント作成・ログイン・実行・提出へのレート制限、実行数の上限と予算分離は運用側で必要。コンパイラ応答や問題ファイルのサイズ上限、取込の同時実行・Git 操作の期限、ジャッジ中の問題更新の分離もアプリでは保証しない。問題リポジトリの書込権限は信頼された管理者だけに与える。
