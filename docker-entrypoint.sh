#!/bin/sh
# 起動時に problems リポジトリ (dist ブランチ) を用意してからアプリを実行する。
# 既に clone 済みなら pull、無ければ clone。problems の置き場は emptyDir/PVC 想定。
set -eu

PROBLEMS_REPO_ROOT="${PROBLEMS_REPO_ROOT:-/data/problems}"
PROBLEMS_REPO_URL="${PROBLEMS_REPO_URL:-https://github.com/kisepichu/qkjudge-problems.git}"
PROBLEMS_REPO_BRANCH="${PROBLEMS_REPO_BRANCH:-dist}"

if [ -d "${PROBLEMS_REPO_ROOT}/.git" ]; then
    # 既存チェックアウトを env で指定した remote/branch に揃える (URL/branch を変えても一貫させる)。
    # problems は read-only な配信内容なので reset --hard でリモートに合わせる。
    echo "[entrypoint] syncing problems repo at ${PROBLEMS_REPO_ROOT} to ${PROBLEMS_REPO_URL}#${PROBLEMS_REPO_BRANCH}"
    git -C "${PROBLEMS_REPO_ROOT}" remote set-url origin "${PROBLEMS_REPO_URL}"
    git -C "${PROBLEMS_REPO_ROOT}" fetch --prune origin "${PROBLEMS_REPO_BRANCH}"
    git -C "${PROBLEMS_REPO_ROOT}" checkout -B "${PROBLEMS_REPO_BRANCH}" "origin/${PROBLEMS_REPO_BRANCH}"
    git -C "${PROBLEMS_REPO_ROOT}" reset --hard "origin/${PROBLEMS_REPO_BRANCH}"
else
    echo "[entrypoint] cloning problems repo (${PROBLEMS_REPO_BRANCH}) into ${PROBLEMS_REPO_ROOT}"
    mkdir -p "${PROBLEMS_REPO_ROOT}"
    git clone -b "${PROBLEMS_REPO_BRANCH}" "${PROBLEMS_REPO_URL}" "${PROBLEMS_REPO_ROOT}"
fi

exec qkjudge
