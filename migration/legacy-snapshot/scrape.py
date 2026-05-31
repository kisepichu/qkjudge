#!/usr/bin/env python3
"""旧 qkjudge 公開 API を全巡回して JSON スナップショットを保存するワンオフスクリプト。

旧サーバー (traP NeoShowcase) はいつ停止されてもおかしくないため、提出履歴・ソース・
テストケース結果・問題文を失わないよう手元 JSON に保存する。本テーブルへは取り込まず、
後段 (TASK-005) の legacy 専用配信に使う素材とする。

設計:
- 標準ライブラリのみ (urllib)。追加依存なし。
- リトライ + レート配慮 (各リクエスト間 sleep, 失敗時は指数バックオフ)。
- 冪等 / 再開可能: 生レスポンスを raw/ に保存し、既取得分は再 fetch せずそれを読む。
  途中で落ちても再実行すれば続きから取得する。
- 公開 API のみを叩くため秘密情報は含まない。スナップショットはコミットして残す。

出力 (このスクリプトと同じディレクトリ):
  raw/problems_list.json          /problems の生レスポンス
  raw/problems/{id}.json          /problems/{id}
  raw/submissions_page_{n}.json   /submissions?page={n}
  raw/submissions/{id}.json       /submissions/{id}
  raw/tasks/{tid}.json            /tasks/{tid}
  problems.json     問題詳細 (statement 含む) の配列
  submissions.json  提出詳細 (source + tasks) の配列
  tasks.json        task 詳細 (input/output/expected/result/memory/cpu_time) の配列
  meta.json         base URL / 取得日時 / 件数
"""

import json
import os
import sys
import time
import urllib.error
import urllib.request
from datetime import datetime, timezone

BASE_URL = os.environ.get("QKJUDGE_LEGACY_BASE", "https://tqk.trap.show/qkjudge")
HERE = os.path.dirname(os.path.abspath(__file__))
RAW = os.path.join(HERE, "raw")

# レート配慮 / リトライ
SLEEP_BETWEEN = float(os.environ.get("QKJUDGE_SLEEP", "0.3"))  # 秒
MAX_RETRIES = int(os.environ.get("QKJUDGE_RETRIES", "5"))
TIMEOUT = int(os.environ.get("QKJUDGE_TIMEOUT", "30"))
USER_AGENT = "qkjudge-legacy-snapshot/1.0 (migration TASK-001)"


def log(msg):
    print(msg, file=sys.stderr, flush=True)


def fetch(path):
    """API から JSON を取得 (リトライ + バックオフ)。404 は None を返す。"""
    url = BASE_URL + path
    delay = 1.0
    last_err = None
    for attempt in range(1, MAX_RETRIES + 1):
        try:
            req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
            with urllib.request.urlopen(req, timeout=TIMEOUT) as resp:
                body = resp.read().decode("utf-8")
            time.sleep(SLEEP_BETWEEN)
            return json.loads(body)
        except urllib.error.HTTPError as e:
            if e.code == 404:
                log(f"  404 {path}")
                return None
            last_err = e
            log(f"  HTTP {e.code} {path} (attempt {attempt}/{MAX_RETRIES})")
        except (urllib.error.URLError, TimeoutError, ConnectionError, json.JSONDecodeError) as e:
            last_err = e
            log(f"  err {path}: {e} (attempt {attempt}/{MAX_RETRIES})")
        if attempt < MAX_RETRIES:
            time.sleep(delay)
            delay = min(delay * 2, 30.0)
    raise RuntimeError(f"failed to fetch {path}: {last_err}")


def cached(rel_path, path):
    """raw/<rel_path> があればそれを読む (再開)。無ければ fetch して保存する。
    fetch が None (404) の場合は保存せず None を返す。"""
    fp = os.path.join(RAW, rel_path)
    if os.path.exists(fp):
        with open(fp, encoding="utf-8") as f:
            return json.load(f)
    data = fetch(path)
    if data is None:
        return None
    os.makedirs(os.path.dirname(fp), exist_ok=True)
    with open(fp, "w", encoding="utf-8") as f:
        json.dump(data, f, ensure_ascii=False, indent=2)
    return data


def write_json(name, data):
    with open(os.path.join(HERE, name), "w", encoding="utf-8") as f:
        json.dump(data, f, ensure_ascii=False, indent=2)


def scrape_problems():
    log("[problems] fetching list")
    listing = cached("problems_list.json", "/problems")
    summaries = listing["problems"] if listing else []
    details = []
    for s in summaries:
        pid = s["id"]
        log(f"[problems] {pid}")
        d = cached(f"problems/{pid}.json", f"/problems/{pid}")
        if d is not None:
            details.append(d)
    return details


def scrape_submissions():
    log("[submissions] page 1")
    first = cached("submissions_page_1.json", "/submissions?page=1")
    pages = first["pages_number"]
    summaries = list(first["submissions"])
    for p in range(2, pages + 1):
        log(f"[submissions] page {p}/{pages}")
        page = cached(f"submissions_page_{p}.json", f"/submissions?page={p}")
        summaries.extend(page["submissions"])

    # id 昇順で安定化 (一覧は DESC)
    sub_ids = sorted({s["id"] for s in summaries})
    details = []
    task_ids = []
    for sid in sub_ids:
        log(f"[submissions] detail {sid}")
        d = cached(f"submissions/{sid}.json", f"/submissions/{sid}")
        if d is None:
            continue
        details.append(d)
        for t in d.get("tasks", []):
            task_ids.append(t["id"])
    return pages, summaries, details, sorted(set(task_ids))


def scrape_tasks(task_ids):
    details = []
    total = len(task_ids)
    for i, tid in enumerate(task_ids, 1):
        log(f"[tasks] {tid} ({i}/{total})")
        d = cached(f"tasks/{tid}.json", f"/tasks/{tid}")
        if d is not None:
            details.append(d)
    return details


def main():
    os.makedirs(RAW, exist_ok=True)
    started = datetime.now(timezone.utc).isoformat()

    problems = scrape_problems()
    pages, sub_summaries, submissions, task_ids = scrape_submissions()
    tasks = scrape_tasks(task_ids)

    write_json("problems.json", problems)
    write_json("submissions.json", submissions)
    write_json("tasks.json", tasks)

    meta = {
        "base_url": BASE_URL,
        "fetched_at_utc": started,
        "finished_at_utc": datetime.now(timezone.utc).isoformat(),
        "counts": {
            "problems": len(problems),
            "submissions_pages_number": pages,
            "submission_summaries": len(sub_summaries),
            "submissions": len(submissions),
            "tasks": len(tasks),
        },
    }
    write_json("meta.json", meta)

    log("")
    log("=== done ===")
    log(json.dumps(meta["counts"], indent=2))


if __name__ == "__main__":
    main()
