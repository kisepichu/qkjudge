# マルチステージビルド: builder(rust) で SQLX_OFFLINE ビルド → runtime(debian-slim) に成果物だけ配置。
# DB 無しでビルドできるよう sqlx-data.json (cargo sqlx prepare 生成物) をコミットして使う。

FROM rust:1.96-bookworm AS builder
WORKDIR /app

# 注: rust:bookworm は buildpack-deps ベースで pkg-config / libssl-dev / git を同梱するため
# builder では apt を叩かない (ミラー不調を避ける)。

# 依存だけ先にビルドしてレイヤキャッシュを効かせる。
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs \
    && cargo build --release --locked \
    && rm -rf src

# 本体ビルド。sqlx-data.json があるため実 DB 不要 (SQLX_OFFLINE=true)。
# legacy-snapshot は legacy_store.rs が include_str! で取り込むため build に必要。
COPY src ./src
COPY sqlx-data.json ./sqlx-data.json
COPY migration/legacy-snapshot ./migration/legacy-snapshot
ENV SQLX_OFFLINE=true
RUN touch src/main.rs && cargo build --release --locked

FROM debian:bookworm-slim AS runtime
# git: problems リポジトリの clone/pull に必要。libssl3: HTTPS。
# 注: apt は HTTPS ミラーから取得する (plain HTTP は大きいインデックスで途中切断されやすいため)。
# HTTPS 検証用に CA 証明書を builder からコピーしてから apt を実行する。
COPY --from=builder /etc/ssl/certs /etc/ssl/certs
RUN sed -i 's|http://deb.debian.org|https://deb.debian.org|g; s|http://security.debian.org|https://security.debian.org|g' \
        /etc/apt/sources.list.d/debian.sources \
    && printf 'Acquire::Retries "10";\n' > /etc/apt/apt.conf.d/99retries \
    && for i in 1 2 3 4 5 6 7 8 9 10; do \
         apt-get update \
         && apt-get install -y --no-install-recommends ca-certificates libssl3 git \
         && break; \
         echo "apt retry $i"; sleep 3; \
       done \
    && command -v git \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/qkjudge /usr/local/bin/qkjudge
COPY docker-entrypoint.sh /usr/local/bin/docker-entrypoint.sh
RUN chmod +x /usr/local/bin/docker-entrypoint.sh

# problems の置き場 (emptyDir/PVC 想定。TASK-003 で接続)。
# PROBLEMS_REPO_ROOT: git clone/pull 先 (末尾スラッシュ無し)。
# PROBLEMS_ROOT: パス連結の接頭辞。コードは `PROBLEMS_ROOT + 問題名` と素朴に連結するため
#   末尾スラッシュ必須 (旧デプロイもスラッシュ付きだった)。
ENV PROBLEMS_REPO_ROOT=/data/problems
ENV PROBLEMS_ROOT=/data/problems/
RUN mkdir -p /data/problems

EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/docker-entrypoint.sh"]
