use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use super::{ChangedFile, DiffDocument, StatusSnapshot, parse_diff, parse_status};

#[derive(Clone, Debug)]
pub struct Repository {
    root: PathBuf,
    git_dir: PathBuf,
    disabled_filters: Vec<String>,
}

#[derive(Debug)]
pub enum GitError {
    InvalidDirectory(PathBuf),
    NotRepository(PathBuf),
    Command { command: String, details: String },
    InvalidPath(String),
}

impl std::fmt::Display for GitError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidDirectory(path) => {
                write!(formatter, "{} is not a directory", path.display())
            }
            Self::NotRepository(path) => write!(
                formatter,
                "{} is not inside a Git repository",
                path.display()
            ),
            Self::Command { command, details } => {
                write!(formatter, "git {command} failed: {details}")
            }
            Self::InvalidPath(path) => write!(formatter, "invalid repository path: {path}"),
        }
    }
}

impl std::error::Error for GitError {}

impl Repository {
    pub fn discover(target: &Path) -> Result<Self, GitError> {
        let target = target
            .canonicalize()
            .map_err(|_| GitError::NotRepository(target.to_path_buf()))?;
        if !target.is_dir() {
            return Err(GitError::InvalidDirectory(target));
        }

        let root_output = git_command(&target, &[])
            .args(["rev-parse", "--show-toplevel"])
            .output()
            .map_err(|error| GitError::Command {
                command: "rev-parse".into(),
                details: error.to_string(),
            })?;
        if !root_output.status.success() {
            return Err(GitError::NotRepository(target));
        }
        let root = output_path(root_output)?;

        let git_output = git_command(&root, &[])
            .args(["rev-parse", "--absolute-git-dir"])
            .output()
            .map_err(|error| GitError::Command {
                command: "rev-parse".into(),
                details: error.to_string(),
            })?;
        if !git_output.status.success() {
            return Err(command_error("rev-parse", git_output));
        }

        let disabled_filters = configured_filter_commands(&root)?;
        Ok(Self {
            root,
            git_dir: output_path(git_output)?,
            disabled_filters,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn git_dir(&self) -> &Path {
        &self.git_dir
    }

    pub fn status(&self) -> Result<StatusSnapshot, GitError> {
        let output = self.run_git(&[
            "status",
            "--porcelain=v2",
            "-z",
            "--untracked-files=all",
            "--renames",
        ])?;
        Ok(StatusSnapshot {
            files: parse_status(&output.stdout),
        })
    }

    pub fn diff(&self, file: &ChangedFile) -> Result<DiffDocument, GitError> {
        let absolute_path = self.root.join(&file.path);
        let output = match file.kind {
            super::FileKind::Untracked => git_command(&self.root, &self.disabled_filters)
                .args([
                    "diff",
                    "--no-index",
                    "--no-ext-diff",
                    "--no-textconv",
                    "--no-color",
                    "--unified=3",
                    "--",
                    "/dev/null",
                ])
                .arg(&absolute_path)
                .output()
                .map_err(|error| GitError::Command {
                    command: "diff".into(),
                    details: error.to_string(),
                })?,
            _ => {
                let path = file
                    .path
                    .to_str()
                    .ok_or_else(|| GitError::InvalidPath(file.path.display().to_string()))?;
                let args = [
                    "diff",
                    "--no-ext-diff",
                    "--no-textconv",
                    "--no-color",
                    "--unified=3",
                    "--find-renames",
                    "HEAD",
                    "--",
                    path,
                ];
                match self.run_git(&args) {
                    Ok(output) => output,
                    Err(_) if !self.has_head() => {
                        // An unborn branch has no HEAD tree. Show its index diff and
                        // working tree diff separately so staged additions still review.
                        let staged = self.run_git(&[
                            "diff",
                            "--no-ext-diff",
                            "--no-textconv",
                            "--no-color",
                            "--unified=3",
                            "--cached",
                            "--",
                            path,
                        ])?;
                        let unstaged = self.run_git(&[
                            "diff",
                            "--no-ext-diff",
                            "--no-textconv",
                            "--no-color",
                            "--unified=3",
                            "--",
                            path,
                        ])?;
                        let mut combined = staged.stdout;
                        combined.extend_from_slice(&unstaged.stdout);
                        Output {
                            status: std::process::ExitStatus::from_raw(0),
                            stdout: combined,
                            stderr: Vec::new(),
                        }
                    }
                    Err(error) => return Err(error),
                }
            }
        };

        if !output.status.success() && output.status.code() != Some(1) {
            return Err(command_error("diff", output));
        }
        Ok(parse_diff(&String::from_utf8_lossy(&output.stdout)))
    }

    fn run_git(&self, args: &[&str]) -> Result<Output, GitError> {
        let output = git_command(&self.root, &self.disabled_filters)
            .args(args)
            .output()
            .map_err(|error| GitError::Command {
                command: args.first().copied().unwrap_or("command").to_owned(),
                details: error.to_string(),
            })?;
        if !output.status.success() {
            return Err(command_error(
                args.first().copied().unwrap_or("command"),
                output,
            ));
        }
        Ok(output)
    }

    fn has_head(&self) -> bool {
        git_command(&self.root, &self.disabled_filters)
            .args(["rev-parse", "--verify", "HEAD"])
            .output()
            .is_ok_and(|output| output.status.success())
    }
}

fn git_command(directory: &Path, disabled_filters: &[String]) -> Command {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(directory)
        .args(["--no-optional-locks", "-c", "core.fsmonitor=false"]);
    for filter in disabled_filters {
        command.args(["-c", filter]);
    }
    command
}

fn configured_filter_commands(root: &Path) -> Result<Vec<String>, GitError> {
    let output = git_command(root, &[])
        .args([
            "config",
            "--null",
            "--name-only",
            "--get-regexp",
            r"^filter\..*\.(clean|smudge|process|required)$",
        ])
        .output()
        .map_err(|error| GitError::Command {
            command: "config".into(),
            details: error.to_string(),
        })?;

    if !output.status.success() && output.status.code() != Some(1) {
        return Err(command_error("config", output));
    }

    Ok(output
        .stdout
        .split(|byte| *byte == 0)
        .filter_map(|key| std::str::from_utf8(key).ok())
        .filter(|key| !key.is_empty())
        .map(|key| {
            if key.ends_with(".required") {
                format!("{key}=false")
            } else {
                format!("{key}=")
            }
        })
        .collect())
}

fn output_path(output: Output) -> Result<PathBuf, GitError> {
    let text = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if text.is_empty() {
        Err(GitError::Command {
            command: "rev-parse".into(),
            details: "Git returned an empty path".into(),
        })
    } else {
        Ok(PathBuf::from(text))
    }
}

fn command_error(command: &str, output: Output) -> GitError {
    GitError::Command {
        command: command.to_owned(),
        details: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::process::Command;

    use super::Repository;

    struct TemporaryRepository(PathBuf);

    impl Drop for TemporaryRepository {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn reads_working_tree_and_untracked_diffs_from_git() {
        let id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("zako-test-{}-{id}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let _temporary_repository = TemporaryRepository(root.clone());

        run_git(&root, &["init", "--quiet"]);
        run_git(&root, &["config", "user.name", "Zako Tests"]);
        run_git(&root, &["config", "user.email", "zako@example.invalid"]);
        std::fs::write(root.join("tracked.rs"), "fn original() {}\n").unwrap();
        run_git(&root, &["add", "tracked.rs"]);
        run_git(&root, &["commit", "--quiet", "-m", "initial"]);

        std::fs::write(root.join("tracked.rs"), "fn updated() {}\n").unwrap();
        std::fs::write(root.join("new.rs"), "fn fresh() {}\n").unwrap();

        let repository = Repository::discover(&root).unwrap();
        let status = repository.status().unwrap();
        assert_eq!(status.files.len(), 2);
        let tracked = status
            .files
            .iter()
            .find(|file| file.display_path == "tracked.rs")
            .unwrap();
        let tracked_diff = repository.diff(tracked).unwrap();
        assert_eq!(tracked_diff.additions, 1);
        assert_eq!(tracked_diff.deletions, 1);

        let untracked = status
            .files
            .iter()
            .find(|file| file.display_path == "new.rs")
            .unwrap();
        let untracked_diff = repository.diff(untracked).unwrap();
        assert_eq!(untracked_diff.additions, 1);
        assert_eq!(untracked_diff.deletions, 0);
    }

    #[test]
    fn does_not_run_repository_configured_filters() {
        let id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("zako-filter-test-{}-{id}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let _temporary_repository = TemporaryRepository(root.clone());

        run_git(&root, &["init", "--quiet"]);
        run_git(&root, &["config", "user.name", "Zako Tests"]);
        run_git(&root, &["config", "user.email", "zako@example.invalid"]);
        std::fs::write(root.join("tracked.rs"), "fn original() {}\n").unwrap();
        std::fs::write(root.join(".gitattributes"), "tracked.rs filter=probe\n").unwrap();
        run_git(&root, &["add", "tracked.rs", ".gitattributes"]);
        run_git(&root, &["commit", "--quiet", "-m", "initial"]);

        let marker = root.join("filter-ran");
        let filter_command = format!("touch {}", marker.display());
        run_git(&root, &["config", "filter.probe.clean", &filter_command]);
        run_git(&root, &["config", "filter.probe.required", "true"]);
        std::fs::write(root.join("tracked.rs"), "fn modified() {}\n").unwrap();

        let repository = Repository::discover(&root).unwrap();
        let status = repository.status().unwrap();
        let tracked = status
            .files
            .iter()
            .find(|file| file.display_path == "tracked.rs")
            .unwrap();
        repository.diff(tracked).unwrap();

        assert!(!marker.exists(), "Git ran the configured clean filter");
    }

    fn run_git(root: &std::path::Path, args: &[&str]) {
        let output = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
