#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineKind {
    Context,
    Addition,
    Deletion,
    Hunk,
    Metadata,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffLine {
    pub old_line: Option<usize>,
    pub new_line: Option<usize>,
    pub kind: LineKind,
    pub text: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DiffDocument {
    pub lines: Vec<DiffLine>,
    pub additions: usize,
    pub deletions: usize,
    pub hunk_lines: Vec<usize>,
}

pub fn parse_diff(diff: &str) -> DiffDocument {
    let mut document = DiffDocument::default();
    let mut old_line = None;
    let mut new_line = None;

    for line in diff.lines() {
        if line.starts_with("@@") {
            let (old, new) = parse_hunk_header(line);
            old_line = Some(old);
            new_line = Some(new);
            document.hunk_lines.push(document.lines.len());
            document.lines.push(DiffLine {
                old_line: None,
                new_line: None,
                kind: LineKind::Hunk,
                text: line.to_owned(),
            });
        } else if line.starts_with("+") && !line.starts_with("+++") {
            document.additions += 1;
            document.lines.push(DiffLine {
                old_line: None,
                new_line,
                kind: LineKind::Addition,
                text: line.to_owned(),
            });
            if let Some(number) = &mut new_line {
                *number += 1;
            }
        } else if line.starts_with('-') && !line.starts_with("---") {
            document.deletions += 1;
            document.lines.push(DiffLine {
                old_line,
                new_line: None,
                kind: LineKind::Deletion,
                text: line.to_owned(),
            });
            if let Some(number) = &mut old_line {
                *number += 1;
            }
        } else if line.starts_with(' ') {
            document.lines.push(DiffLine {
                old_line,
                new_line,
                kind: LineKind::Context,
                text: line.to_owned(),
            });
            if let Some(number) = &mut old_line {
                *number += 1;
            }
            if let Some(number) = &mut new_line {
                *number += 1;
            }
        } else {
            document.lines.push(DiffLine {
                old_line: None,
                new_line: None,
                kind: LineKind::Metadata,
                text: line.to_owned(),
            });
        }
    }
    document
}

fn parse_hunk_header(header: &str) -> (usize, usize) {
    let mut fields = header.split_whitespace();
    let _at_at = fields.next();
    let range = fields.next().unwrap_or("-0,0");
    let old = range
        .trim_start_matches('-')
        .split(',')
        .next()
        .and_then(|number| number.parse().ok())
        .unwrap_or(0);
    let new = fields
        .next()
        .unwrap_or("+0,0")
        .trim_start_matches('+')
        .split(',')
        .next()
        .and_then(|number| number.parse().ok())
        .unwrap_or(0);
    (old, new)
}

#[cfg(test)]
mod tests {
    use super::{LineKind, parse_diff};

    #[test]
    fn parses_hunks_line_numbers_and_change_counts() {
        let diff = "diff --git a/src/lib.rs b/src/lib.rs\nindex 123..456 100644\n--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -7,3 +7,4 @@ fn main() {\n context\n-old()\n+new()\n+extra()\n }";
        let parsed = parse_diff(diff);
        assert_eq!(parsed.additions, 2);
        assert_eq!(parsed.deletions, 1);
        assert_eq!(parsed.hunk_lines, vec![4]);
        assert_eq!(parsed.lines[5].old_line, Some(7));
        assert_eq!(parsed.lines[5].new_line, Some(7));
        assert_eq!(parsed.lines[6].old_line, Some(8));
        assert_eq!(parsed.lines[7].new_line, Some(8));
        assert_eq!(parsed.lines[6].kind, LineKind::Deletion);
    }

    #[test]
    fn recognizes_binary_metadata_without_counting_changes() {
        let parsed = parse_diff("Binary files a/image.png and b/image.png differ");
        assert_eq!(parsed.additions, 0);
        assert_eq!(parsed.deletions, 0);
        assert_eq!(parsed.lines[0].kind, LineKind::Metadata);
    }
}
