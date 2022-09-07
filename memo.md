# 開発めも

## gps

手元と dev と master がある
- http://localhost:8080
  - https 通信うまくできず..ので手元ではログインせずに試してる
  - db のバージョンが違くて? 手元だけで動く sql 文などがある<br>
  `INSERT {...} RETURNING id;` など 原因こんど調べる<br>
	手元: `0.6.8-MariaDB-1:10.6.8+maria~bionic`, Showcase: `10.1.36-MariaDB`
- https://dev_tqk_qkjudge.trap.games/
  - dev ブランチに最初から割り当てられている url では https 通信できなくて、showcase の cname 追加機能使うと行ける
- https://tqk.trap.show/qkjudge


## 使っているツールなど
- Showcase: traP の Paas 基盤。動かす
- Compiler API: コンパイルや実行をしてくれる外部アプリ 無料枠一日 200 回 セキュリティ的にむずそうな部分を任せている いつか自分でチャレンジ
- Rust: 言語 ライブラリ選択:
  - actix-web: web サーバーのフレームワーク
  - sqlx: db 操作するやつ sql 文をそのまま書く感じでわかりやすい！<br>
  手元でデータベース起動してないとビルドできなくて、そういう column があるかや型などチェックしてくれる NOT NULL つけ忘れると Option 型になるのでハマるの注意
  - reqwest: http クライアント Compiler API に接続するため
- sqldef: db マイグレーションのツール テーブル定義の sql を書くだけでわかりやすい！ 詳しくは下


## db のスキーマのマイグレーションについて

sqldef を使う。テーブル定義をそのまま書いた sql (例えば、[migrations/v0.3.2.sql](https://github.com/tqkoh/qkjudge/blob/dev/migrations/v0.3.2.sql)) を用意して、以下のコマンドを実行すれば、今の構造との差分を自動で計算してくれる。言語によらず使えるの好き

```bash
mysqldef -uroot qkjudge < migrations/v0.3.2.sql
```

- 問題点 1: バージョン管理の機能はない！(一つの sql ファイルを編集するべきかもしれないが、下の理由によりだるい)
- 問題点 2: Showcase は `mysql` コマンドなどが直接叩けないので(https://phpmyadmin.trap.show/ から操作する)、当然 sqldef も動かせない！

解決: <br>
自分でバージョン管理する。`migrations` テーブルを作り、今のバージョンを保存しておく。<br>
`mysqldef` は実行した差分の sql も標準出力するので、それを実行し、そのあとにバージョンを書き込む。<br>
Showcase 上では、バージョンを確認し、手元で実行した差分 SQL 実行とバージョン設定をする。<br>
<br>
[↑ を楽にするスクリプト](https://github.com/tqkoh/qkjudge/blob/dev/migrations/migrate.sh) を使うと、v0.3.1 から v0.3.2 に変更する場合、手元で

```bash
$ cd migrations
$ source migrate.sh v0.3.2
current version: v0.3.1
current version: v0.3.2
$ 
```

をして、(showcase 側のバージョンが v0.3.1 なことを確認して、)<br>
生成された [migrations/out/v0.3.1_to_v0.3.2.sql](https://github.com/tqkoh/qkjudge/blob/dev/migrations/out/v0.3.1_to_v0.3.2.sql) の内容を Showcase 側(https://phpmyadmin.trap.show/)から実行すればいい。

~~めも: v0.3.0 以降しか対応してないのでそれより前を渡さないように~~
更新先が v1.0.0 以降しかできないように除外した

ナンもシランのでもっといい方法あるか調べぶ
