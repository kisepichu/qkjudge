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
- sqldef: db マイグレーションのツール 説明:

テーブル定義の sql を書くだけでわかりやすい！

```bash
mysqldef -uroot qkjudge < migrations/v0.2.2.sql > "migrations/out/v0.2.1_to_v0.2.2.sql"
```

こういう感じで実行すると実際に変更するときの sql が自動生成されるので、showcase の dev や master の環境の db にはそれを入力する(https://phpmyadmin.trap.show/ から。showcase のアプリのページからユーザー情報をみれる)
