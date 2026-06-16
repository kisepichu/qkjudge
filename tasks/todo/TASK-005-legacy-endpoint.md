# TASK-005: legacy 提出の read-only 配信エンドポイント (Phase 4)

## 参照

- `docs/MIGRATION.md`「レガシーデータ」
- TASK-001 のスナップショット (`migration/legacy-snapshot/`)
- `src/routes/get_submissions.rs` `get_submissions_sid.rs` (新提出側のページング)
- DB スキーマ `migrations/v1.2.3.sql` (submissions.author→users / problem_id→problems の FK)

## 概要

TASK-001 のスナップショットを、新サーバーから read-only で配信する。FK 制約を避けるため
**本テーブルには取り込まず** legacy 専用ストアから返す。UI (TASK-006) は提出一覧の末尾(=新提出が
尽きた後)に legacy を表示し、author に `[legacy]` prefix を付ける。

## 設計メモ

- ストア: スナップショット JSON をイメージに同梱 (read-only)。件数は小さい (~数十提出/数百 task) ので
  起動時メモリ読み込みで十分。
- エンドポイント案:
  - `GET /legacy/submissions?page=N` — legacy summary のページング (新側と同形の JSON)。
  - `GET /legacy/submissions/{id}` — source + tasks。
  - `GET /legacy/tasks/{tid}` — 詳細。
  - id は新提出と衝突しないよう legacy 名前空間を分離 (例: 別系列 id か `legacy-` 接頭、UI と取り決め)。
- ページング合成方針: 新提出の総ページの後に legacy ページを連結 (UI 側で「最後まで来たら legacy」
  を実現できるレスポンスを返す)。具体的なページ番号設計は UI(TASK-006) と合わせる。
- 認証: 新側 submissions と同じ公開度 (未ログイン可)。

## チェックリスト

- [ ] スナップショットを読み込む legacy ストア (起動時ロード)
- [ ] `GET /legacy/submissions` / `/legacy/submissions/{id}` / `/legacy/tasks/{tid}` を実装
- [ ] author 表示は UI 側 `[legacy]` 前提 (サーバーは生値、もしくは取り決めた形)
- [ ] 新提出との id 衝突回避を UI と合意・実装
- [ ] ユニットテスト (ページング境界・存在しない id)

## 完了条件

- [ ] 旧サーバーが停止していても legacy 提出/ソース/タスクが参照できる
- [ ] 新提出のページングを壊さず、末尾に legacy が続く

## 作業ログ

- 2026-05-31: タスク生成。
