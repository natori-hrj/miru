use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileKind {
    Modified,
    Added,
    Deleted,
    Renamed,
    Untracked,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangedFile {
    pub path: PathBuf,
    pub display_path: String,
    pub marker: char,
    pub kind: FileKind,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StatusSnapshot {
    pub files: Vec<ChangedFile>,
}

pub fn parse_status(input: &[u8]) -> Vec<ChangedFile> {
    let mut records = input.split(|byte| *byte == 0).peekable();
    let mut files = Vec::new();

    while let Some(record) = records.next() {
        if record.is_empty() || record.starts_with(b"# ") {
            continue;
        }
        let Ok(record) = std::str::from_utf8(record) else {
            continue;
        };

        let (marker, kind, path) = if let Some(rest) = record.strip_prefix("? ") {
            ('?', FileKind::Untracked, rest)
        } else if let Some(rest) = record.strip_prefix("1 ") {
            parse_status_record(rest, 7)
        } else if let Some(rest) = record.strip_prefix("2 ") {
            let (marker, kind, path) = parse_status_record(rest, 8);
            let _original_path = records.next();
            (marker, kind, path)
        } else if let Some(rest) = record.strip_prefix("u ") {
            parse_status_record(rest, 8)
        } else {
            continue;
        };

        if path.is_empty() {
            continue;
        }
        files.push(ChangedFile {
            path: PathBuf::from(path),
            display_path: path.to_owned(),
            marker,
            kind,
        });
    }

    files.sort_by(|left, right| left.display_path.cmp(&right.display_path));
    files
}

fn parse_status_record(record: &str, metadata_fields: usize) -> (char, FileKind, &str) {
    let mut parts = record.splitn(metadata_fields + 1, ' ');
    let xy = parts.next().unwrap_or("M.");
    for _ in 1..metadata_fields {
        parts.next();
    }
    let path = parts.next().unwrap_or_default();
    let x = xy.as_bytes().first().copied().unwrap_or(b'.') as char;
    let y = xy.as_bytes().get(1).copied().unwrap_or(b'.') as char;
    let status = if x == 'R' || y == 'R' || x == 'C' || y == 'C' {
        ('R', FileKind::Renamed)
    } else if x == 'A' || y == 'A' {
        ('A', FileKind::Added)
    } else if x == 'D' || y == 'D' {
        ('D', FileKind::Deleted)
    } else {
        ('M', FileKind::Modified)
    };
    (status.0, status.1, path)
}

#[cfg(test)]
mod tests {
    use super::{FileKind, parse_status};

    #[test]
    fn parses_porcelain_v2_statuses_and_spaces_in_paths() {
        let output = b"# branch.oid abc\0? src/new file.rs\0\x31 .M N... 100644 100644 100644 a b src/service.rs\0\x31 A. N... 000000 100644 100644 000 123 src/auth.rs\0\x31 .D N... 100644 000000 000000 abc 000 src/old.rs\0";
        let files = parse_status(output);
        assert_eq!(files.len(), 4);
        assert_eq!(files[0].display_path, "src/auth.rs");
        assert_eq!(files[0].marker, 'A');
        assert_eq!(files[1].display_path, "src/new file.rs");
        assert_eq!(files[1].kind, FileKind::Untracked);
        assert_eq!(files[2].display_path, "src/old.rs");
        assert_eq!(files[2].marker, 'D');
        assert_eq!(files[3].display_path, "src/service.rs");
        assert_eq!(files[3].marker, 'M');
    }

    #[test]
    fn consumes_the_second_nul_field_for_a_rename() {
        let output = b"2 R. N... 100644 100644 100644 abc def R100 src/new.rs\0src/old.rs\0";
        let files = parse_status(output);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].display_path, "src/new.rs");
        assert_eq!(files[0].marker, 'R');
        assert_eq!(files[0].kind, FileKind::Renamed);
    }
}
