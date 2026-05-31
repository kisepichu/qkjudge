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

## 開発セットアップ

ツールは [mise](https://mise.jdx.dev/) で管理している (`mise.toml`)。Rust ツールチェイン
(cargo / clippy / rustfmt) は別途必要。

```sh
mise install        # gitleaks / prek を入れる
prek install        # git フック (pre-commit / pre-push) を有効化
```

### コミット前チェック (prek)

[prek](https://github.com/j178/prek) (pre-commit 互換) で `.pre-commit-config.yaml` のフックを走らせる。
`prek install` 後はコミット時に自動で実行される。

- **commit 時**: trailing-whitespace / end-of-file-fixer / check-added-large-files / check-yaml /
  check-merge-conflict、`cargo fmt` (差分は自動修正 → 再 add)、**gitleaks** (secret 誤コミット防止)。
- **push 時**: `cargo clippy -D warnings` を回す予定。ただし本クレートは `sqlx::query!` マクロを使い
  コンパイルに DB / sqlx offline キャッシュが要るため、**sqlx offline 化 (TASK-002) まで無効化**している。

全ファイルに手動で走らせる:

```sh
prek run --all-files
```

> secret (DB creds / JDoodle / tunnel token / kubeconfig 等) はコミットしない。gitleaks が
> ステージ済み差分を走査して検知・ブロックする。ドキュメント用のサンプル値は
> `# gitleaks:allow` コメントか `.gitleaksignore` で個別に許可する。
