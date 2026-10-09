use sha2::{Digest, Sha256};

use crate::git::{DiffDocument, DiffLine, LineKind};

use super::{DiffSide, ReviewComment};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommentAnchor {
    pub side: DiffSide,
    pub start_line: usize,
    pub end_line: usize,
    pub anchor_hash: String,
    pub anchor_line_count: usize,
}

pub fn anchor_for_selection(
    document: &DiffDocument,
    side: DiffSide,
    display_start: usize,
    display_end: usize,
) -> Option<CommentAnchor> {
    let selected: Vec<_> = document
        .lines
        .iter()
        .enumerate()
        .filter(|(index, line)| {
            *index >= display_start && *index <= display_end && side_line(line, side).is_some()
        })
        .collect();
    let first = selected.first()?.1;
    let last = selected.last()?.1;
    let start_line = side_line(first, side)?.0;
    let end_line = side_line(last, side)?.0;
    if selected.len() != end_line.saturating_sub(start_line).saturating_add(1)
        || selected
            .windows(2)
            .any(|pair| pair[1].0 != pair[0].0.saturating_add(1))
    {
        return None;
    }
    Some(CommentAnchor {
        side,
        start_line,
        end_line,
        anchor_hash: hash_lines(selected.iter().map(|(_, line)| line.text.as_str())),
        anchor_line_count: selected.len(),
    })
}

pub fn reconcile_file_comments(
    comments: &mut [ReviewComment],
    file: &str,
    document: &DiffDocument,
) {
    for comment in comments.iter_mut().filter(|comment| comment.file == file) {
        reconcile_comment(comment, document);
    }
}

pub fn reconcile_comment(comment: &mut ReviewComment, document: &DiffDocument) {
    let candidates: Vec<_> = document
        .lines
        .iter()
        .filter_map(|line| side_line(line, comment.side).map(|(number, _)| (number, line)))
        .collect();
    let count = comment.anchor_line_count;
    if count == 0 || candidates.len() < count {
        comment.stale = true;
        return;
    }

    let matching: Vec<_> = candidates
        .windows(count)
        .filter(|window| {
            let contiguous = window
                .windows(2)
                .all(|pair| pair[1].0 == pair[0].0.saturating_add(1));
            contiguous
                && hash_lines(window.iter().map(|(_, line)| line.text.as_str()))
                    == comment.anchor_hash
        })
        .collect();

    if matching.len() == 1 {
        let window = matching[0];
        comment.start_line = window[0].0;
        comment.end_line = window[window.len() - 1].0;
        comment.stale = false;
    } else {
        comment.stale = true;
    }
}

fn side_line(line: &DiffLine, side: DiffSide) -> Option<(usize, &DiffLine)> {
    match (side, line.kind) {
        (DiffSide::Old, LineKind::Deletion) => line.old_line.map(|number| (number, line)),
        (DiffSide::New, LineKind::Addition) => line.new_line.map(|number| (number, line)),
        _ => None,
    }
}

fn hash_lines<'a>(lines: impl IntoIterator<Item = &'a str>) -> String {
    let mut digest = Sha256::new();
    for line in lines {
        digest.update(line.as_bytes());
        digest.update([0]);
    }
    digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{anchor_for_selection, reconcile_comment};
    use crate::git::{DiffDocument, DiffLine, LineKind};
    use crate::review::{DiffSide, ReviewComment};

    fn comment(anchor: &super::CommentAnchor) -> ReviewComment {
        ReviewComment {
            id: "id".into(),
            file: "src/lib.rs".into(),
            start_line: anchor.start_line,
            end_line: anchor.end_line,
            side: anchor.side,
            body: "Review this".into(),
            resolved: false,
            stale: false,
            anchor_hash: anchor.anchor_hash.clone(),
            anchor_line_count: anchor.anchor_line_count,
            created_at: 1,
            updated_at: 1,
        }
    }

    #[test]
    fn relocates_an_unchanged_anchor_after_line_numbers_shift() {
        let original = DiffDocument {
            lines: vec![DiffLine {
                old_line: None,
                new_line: Some(11),
                kind: LineKind::Addition,
                text: "+target()".into(),
            }],
            ..DiffDocument::default()
        };
        let anchor = anchor_for_selection(&original, DiffSide::New, 0, 0).unwrap();
        let mut comment = comment(&anchor);
        let updated = DiffDocument {
            lines: vec![DiffLine {
                old_line: None,
                new_line: Some(17),
                kind: LineKind::Addition,
                text: "+target()".into(),
            }],
            ..DiffDocument::default()
        };
        reconcile_comment(&mut comment, &updated);
        assert_eq!(comment.start_line, 17);
        assert!(!comment.stale);
    }

    #[test]
    fn marks_an_anchor_stale_when_its_text_disappears() {
        let original = DiffDocument {
            lines: vec![DiffLine {
                old_line: None,
                new_line: Some(11),
                kind: LineKind::Addition,
                text: "+target()".into(),
            }],
            ..DiffDocument::default()
        };
        let anchor = anchor_for_selection(&original, DiffSide::New, 0, 0).unwrap();
        let mut comment = comment(&anchor);
        let updated = DiffDocument::default();
        reconcile_comment(&mut comment, &updated);
        assert!(comment.stale);
    }

    #[test]
    fn anchors_and_relocates_a_contiguous_line_range() {
        let original = DiffDocument {
            lines: vec![
                DiffLine {
                    old_line: None,
                    new_line: Some(11),
                    kind: LineKind::Addition,
                    text: "+first()".into(),
                },
                DiffLine {
                    old_line: None,
                    new_line: Some(12),
                    kind: LineKind::Addition,
                    text: "+second()".into(),
                },
            ],
            ..DiffDocument::default()
        };
        let anchor = anchor_for_selection(&original, DiffSide::New, 0, 1).unwrap();
        assert_eq!((anchor.start_line, anchor.end_line), (11, 12));
        assert_eq!(anchor.anchor_line_count, 2);

        let mut comment = comment(&anchor);
        let updated = DiffDocument {
            lines: vec![
                DiffLine {
                    old_line: None,
                    new_line: Some(17),
                    kind: LineKind::Addition,
                    text: "+first()".into(),
                },
                DiffLine {
                    old_line: None,
                    new_line: Some(18),
                    kind: LineKind::Addition,
                    text: "+second()".into(),
                },
            ],
            ..DiffDocument::default()
        };
        reconcile_comment(&mut comment, &updated);
        assert_eq!((comment.start_line, comment.end_line), (17, 18));
        assert!(!comment.stale);
    }

    #[test]
    fn rejects_ranges_that_skip_changed_side_lines() {
        let document = DiffDocument {
            lines: vec![
                DiffLine {
                    old_line: None,
                    new_line: Some(11),
                    kind: LineKind::Addition,
                    text: "+first()".into(),
                },
                DiffLine {
                    old_line: Some(11),
                    new_line: Some(12),
                    kind: LineKind::Context,
                    text: " context".into(),
                },
                DiffLine {
                    old_line: None,
                    new_line: Some(13),
                    kind: LineKind::Addition,
                    text: "+second()".into(),
                },
            ],
            ..DiffDocument::default()
        };
        assert!(anchor_for_selection(&document, DiffSide::New, 0, 2).is_none());
    }
}
