#!/bin/sh
# 起動時に problems リポジトリ (dist ブランチ) を用意してからアプリを実行する。
# 既に clone 済みなら pull、無ければ clone。problems の置き場は emptyDir/PVC 想定。
set -eu

PROBLEMS_REPO_ROOT="${PROBLEMS_REPO_ROOT:-/data/problems}"
PROBLEMS_REPO_URL="${PROBLEMS_REPO_URL:-https://github.com/kisepichu/qkjudge-problems.git}"
PROBLEMS_REPO_BRANCH="${PROBLEMS_REPO_BRANCH:-dist}"

if [ -d "${PROBLEMS_REPO_ROOT}/.git" ]; then
    echo "[entrypoint] updating problems repo at ${PROBLEMS_REPO_ROOT}"
    git -C "${PROBLEMS_REPO_ROOT}" pull --rebase
else
    echo "[entrypoint] cloning problems repo (${PROBLEMS_REPO_BRANCH}) into ${PROBLEMS_REPO_ROOT}"
    mkdir -p "${PROBLEMS_REPO_ROOT}"
    git clone -b "${PROBLEMS_REPO_BRANCH}" "${PROBLEMS_REPO_URL}" "${PROBLEMS_REPO_ROOT}"
fi

exec qkjudge
