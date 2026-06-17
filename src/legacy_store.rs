use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;

// JSON files are embedded at compile time so the binary works without the migration/ directory.
const SUBMISSIONS_JSON: &str = include_str!("../migration/legacy-snapshot/submissions.json");
const TASKS_JSON: &str = include_str!("../migration/legacy-snapshot/tasks.json");

pub const PER_PAGE: i32 = 10;

#[derive(Deserialize, Clone)]
pub struct LegacyTaskRef {
    pub id: i32,
    pub result: String,
}

#[derive(Deserialize, Clone)]
pub struct LegacySubmission {
    pub id: i32,
    pub date: String,
    pub author: String,
    pub problem_id: i32,
    pub problem_title: String,
    pub testcase_num: i32,
    pub tasks: Vec<LegacyTaskRef>,
    pub result: String,
    pub language_id: i32,
    pub source: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct LegacyTask {
    pub id: i32,
    pub submission_id: i32,
    pub input: String,
    pub output: String,
    pub expected: String,
    pub result: String,
    pub memory: String,
    pub cpu_time: String,
}

pub struct LegacyStore {
    submissions: Vec<LegacySubmission>,
    submissions_by_id: HashMap<i32, usize>,
    tasks_by_id: HashMap<i32, LegacyTask>,
}

impl LegacyStore {
    pub fn from_strs(submissions_json: &str, tasks_json: &str) -> Result<Self, serde_json::Error> {
        let mut submissions: Vec<LegacySubmission> = serde_json::from_str(submissions_json)?;
        let tasks: Vec<LegacyTask> = serde_json::from_str(tasks_json)?;

        submissions.sort_by_key(|s| std::cmp::Reverse(s.id));

        let submissions_by_id = submissions
            .iter()
            .enumerate()
            .map(|(idx, s)| (s.id, idx))
            .collect();
        let tasks_by_id = tasks.into_iter().map(|t| (t.id, t)).collect();

        Ok(Self {
            submissions,
            submissions_by_id,
            tasks_by_id,
        })
    }

    pub fn total_count(&self) -> i32 {
        self.submissions.len() as i32
    }

    pub fn pages_number(&self, per_page: i32) -> i32 {
        let total = self.total_count();
        if total == 0 || per_page <= 0 {
            return 0;
        }
        (total + per_page - 1) / per_page
    }

    /// Returns the slice for the requested 1-based page. Returns empty slice if `page` is out of
    /// range; the handler is expected to validate `page >= 1` before calling.
    pub fn page(&self, page: i32, per_page: i32) -> &[LegacySubmission] {
        if page <= 0 || per_page <= 0 {
            return &[];
        }
        let start = ((page - 1) * per_page) as usize;
        if start >= self.submissions.len() {
            return &[];
        }
        let end = (start + per_page as usize).min(self.submissions.len());
        &self.submissions[start..end]
    }

    pub fn submission(&self, id: i32) -> Option<&LegacySubmission> {
        self.submissions_by_id
            .get(&id)
            .and_then(|&idx| self.submissions.get(idx))
    }

    pub fn task(&self, id: i32) -> Option<&LegacyTask> {
        self.tasks_by_id.get(&id)
    }
}

static STORE: OnceLock<LegacyStore> = OnceLock::new();

pub fn global() -> &'static LegacyStore {
    STORE.get_or_init(|| {
        LegacyStore::from_strs(SUBMISSIONS_JSON, TASKS_JSON)
            .expect("embedded legacy snapshot must deserialize")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> LegacyStore {
        LegacyStore::from_strs(SUBMISSIONS_JSON, TASKS_JSON).expect("legacy snapshot deserializes")
    }

    #[test]
    fn loads_embedded_snapshot() {
        let s = store();
        assert_eq!(s.total_count(), 58);
        assert_eq!(s.pages_number(PER_PAGE), 6);
    }

    #[test]
    fn page_one_is_newest_first() {
        let s = store();
        let p1 = s.page(1, PER_PAGE);
        assert_eq!(p1.len(), 10);
        assert_eq!(p1[0].id, 58);
        assert_eq!(p1[9].id, 49);
    }

    #[test]
    fn last_page_holds_remainder() {
        let s = store();
        let last = s.page(6, PER_PAGE);
        assert_eq!(last.len(), 8);
        assert_eq!(last[0].id, 8);
        assert_eq!(last[7].id, 1);
    }

    #[test]
    fn page_past_end_is_empty() {
        let s = store();
        assert!(s.page(7, PER_PAGE).is_empty());
        assert!(s.page(100, PER_PAGE).is_empty());
    }

    #[test]
    fn page_zero_or_negative_is_empty() {
        let s = store();
        assert!(s.page(0, PER_PAGE).is_empty());
        assert!(s.page(-1, PER_PAGE).is_empty());
    }

    #[test]
    fn pages_number_handles_zero_per_page() {
        let s = store();
        assert_eq!(s.pages_number(0), 0);
        assert_eq!(s.pages_number(-1), 0);
    }

    #[test]
    fn submission_lookup_hit_and_miss() {
        let s = store();
        let one = s.submission(1).expect("submission id=1 exists");
        assert_eq!(one.id, 1);
        assert_eq!(one.problem_title, "A+B");
        assert_eq!(one.tasks.len(), 3);
        assert!(s.submission(0).is_none());
        assert!(s.submission(9999).is_none());
    }

    #[test]
    fn task_lookup_hit_and_miss() {
        let s = store();
        let t = s.task(1).expect("task id=1 exists");
        assert_eq!(t.id, 1);
        assert_eq!(t.submission_id, 1);
        assert!(s.task(0).is_none());
        assert!(s.task(99999).is_none());
    }
}
