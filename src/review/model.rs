use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffSide {
    Old,
    New,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct ReviewComment {
    pub id: String,
    pub file: String,
    pub start_line: usize,
    pub end_line: usize,
    pub side: DiffSide,
    pub body: String,
    pub resolved: bool,
    #[serde(default)]
    pub stale: bool,
    pub anchor_hash: String,
    pub anchor_line_count: usize,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct ReviewDocument {
    pub version: u32,
    pub repository_id: String,
    pub repository_path: String,
    pub comments: Vec<ReviewComment>,
}

impl ReviewDocument {
    pub fn unresolved_count(&self) -> usize {
        self.comments
            .iter()
            .filter(|comment| !comment.resolved)
            .count()
    }

    pub fn resolved_count(&self) -> usize {
        self.comments
            .iter()
            .filter(|comment| comment.resolved)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::{DiffSide, ReviewComment, ReviewDocument};

    #[test]
    fn serde_round_trip_preserves_review_comments() {
        let document = ReviewDocument {
            version: 1,
            repository_id: "repo-id".into(),
            repository_path: "/repo".into(),
            comments: vec![ReviewComment {
                id: "comment-id".into(),
                file: "src/lib.rs".into(),
                start_line: 4,
                end_line: 5,
                side: DiffSide::Old,
                body: "Review this range".into(),
                resolved: false,
                stale: true,
                anchor_hash: "hash".into(),
                anchor_line_count: 2,
                created_at: 1,
                updated_at: 2,
            }],
        };
        let encoded = serde_json::to_vec(&document).unwrap();
        let decoded: ReviewDocument = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, document);
    }

    #[test]
    fn counts_resolved_and_unresolved_comments() {
        let comment = |id: &str, resolved| ReviewComment {
            id: id.into(),
            file: "file.rs".into(),
            start_line: 1,
            end_line: 1,
            side: DiffSide::New,
            body: "note".into(),
            resolved,
            stale: false,
            anchor_hash: "hash".into(),
            anchor_line_count: 1,
            created_at: 1,
            updated_at: 1,
        };
        let document = ReviewDocument {
            version: 1,
            repository_id: "repo-id".into(),
            repository_path: "/repo".into(),
            comments: vec![comment("open", false), comment("closed", true)],
        };
        assert_eq!(document.unresolved_count(), 1);
        assert_eq!(document.resolved_count(), 1);
    }
}
