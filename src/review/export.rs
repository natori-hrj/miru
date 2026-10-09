use std::collections::BTreeMap;

use super::{DiffSide, ReviewDocument};

pub fn line_range(start_line: usize, end_line: usize) -> String {
    if start_line == end_line {
        format!("Line {start_line}")
    } else {
        format!("Lines {start_line}-{end_line}")
    }
}

pub fn display_file_path(path: &str) -> String {
    path.chars()
        .map(|character| match character {
            '\n' => "\\n".to_owned(),
            '\r' => "\\r".to_owned(),
            '\t' => "\\t".to_owned(),
            character if character.is_control() => {
                format!("\\u{{{:x}}}", character as u32)
            }
            character => character.to_string(),
        })
        .collect()
}

pub fn export_markdown(document: &ReviewDocument) -> String {
    let mut by_file: BTreeMap<&str, Vec<_>> = BTreeMap::new();
    for comment in document.comments.iter().filter(|comment| !comment.resolved) {
        by_file.entry(&comment.file).or_default().push(comment);
    }

    let mut output = String::from("Please address the following review comments.\n");
    for (file, mut comments) in by_file {
        comments.sort_by_key(|comment| (comment.start_line, comment.end_line));
        output.push_str(&format!("\n## {}\n", format_file_heading(file)));
        for comment in comments {
            let side = if comment.side == DiffSide::Old {
                "Old side · "
            } else {
                ""
            };
            let stale = if comment.stale {
                " · stale location"
            } else {
                ""
            };
            output.push_str(&format!(
                "\n### {side}{}{stale}\n\n{}\n",
                line_range(comment.start_line, comment.end_line),
                comment.body.trim(),
            ));
        }
    }
    output
}

fn format_file_heading(path: &str) -> String {
    let sanitized = display_file_path(path);
    if sanitized != path
        || path
            .chars()
            .any(|character| matches!(character, '`' | '\\' | '[' | ']' | '<' | '>'))
    {
        inline_code(&sanitized)
    } else {
        path.to_owned()
    }
}

fn inline_code(value: &str) -> String {
    let mut sanitized = String::new();
    for character in value.chars() {
        match character {
            '\n' => sanitized.push_str("\\n"),
            '\r' => sanitized.push_str("\\r"),
            character if character.is_control() => {
                sanitized.push_str(&format!("\\u{{{:x}}}", character as u32));
            }
            character => sanitized.push(character),
        }
    }
    let mut longest_backtick_run = 0;
    let mut current_backtick_run: usize = 0;
    for character in sanitized.chars() {
        if character == '`' {
            current_backtick_run += 1;
            longest_backtick_run = longest_backtick_run.max(current_backtick_run);
        } else {
            current_backtick_run = 0;
        }
    }
    let delimiter = "`".repeat(longest_backtick_run.saturating_add(1));
    format!("{delimiter} {sanitized} {delimiter}")
}

#[cfg(test)]
mod tests {
    use super::{display_file_path, export_markdown, line_range};
    use crate::review::{DiffSide, ReviewComment, ReviewDocument};

    #[test]
    fn formats_single_and_multiple_line_ranges() {
        assert_eq!(line_range(43, 43), "Line 43");
        assert_eq!(line_range(81, 87), "Lines 81-87");
    }

    #[test]
    fn exports_only_unresolved_comments_grouped_by_file() {
        let document = ReviewDocument {
            version: 1,
            repository_id: "repo".into(),
            repository_path: "/repo".into(),
            comments: vec![
                ReviewComment {
                    id: "a".into(),
                    file: "src/service.rs".into(),
                    start_line: 43,
                    end_line: 43,
                    side: DiffSide::New,
                    body: "Wrap this error with context.".into(),
                    resolved: false,
                    stale: false,
                    anchor_hash: "hash".into(),
                    anchor_line_count: 1,
                    created_at: 1,
                    updated_at: 1,
                },
                ReviewComment {
                    id: "b".into(),
                    file: "src/old.rs".into(),
                    start_line: 8,
                    end_line: 9,
                    side: DiffSide::Old,
                    body: "Already resolved.".into(),
                    resolved: true,
                    stale: false,
                    anchor_hash: "hash".into(),
                    anchor_line_count: 2,
                    created_at: 1,
                    updated_at: 1,
                },
            ],
        };
        let markdown = export_markdown(&document);
        assert!(markdown.contains("## src/service.rs"));
        assert!(markdown.contains("### Line 43"));
        assert!(markdown.contains("Wrap this error with context."));
        assert!(!markdown.contains("src/old.rs"));
    }

    #[test]
    fn file_names_cannot_inject_markdown_sections() {
        let mut document = ReviewDocument {
            version: 1,
            repository_id: "repo".into(),
            repository_path: "/repo".into(),
            comments: Vec::new(),
        };
        let comment = ReviewComment {
            id: "a".into(),
            file: "src/evil\n## forged`name.rs".into(),
            start_line: 1,
            end_line: 1,
            side: DiffSide::New,
            body: "Review this".into(),
            resolved: false,
            stale: false,
            anchor_hash: "hash".into(),
            anchor_line_count: 1,
            created_at: 1,
            updated_at: 1,
        };
        document.comments.push(comment);
        let markdown = export_markdown(&document);
        assert!(!markdown.contains("\n## forged"));
        assert!(markdown.contains("\\n## forged"));
        assert!(markdown.contains("`` src/evil"));
        assert_eq!(
            display_file_path("src/control\u{1b}.rs"),
            "src/control\\u{1b}.rs"
        );
    }
}
