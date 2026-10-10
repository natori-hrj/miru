use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{
    fs::Permissions,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
};

use sha2::{Digest, Sha256};

use super::ReviewDocument;

const REVIEW_VERSION: u32 = 1;

#[derive(Debug)]
pub enum ReviewError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    InvalidJson {
        path: PathBuf,
        source: serde_json::Error,
    },
    UnsupportedVersion {
        path: PathBuf,
        version: u32,
    },
    RepositoryMismatch {
        path: PathBuf,
    },
    MissingHomeDirectory,
    InvalidRepositoryPath(PathBuf),
    StorageInsideRepository {
        path: PathBuf,
        repository: PathBuf,
    },
    StorageSymlink(PathBuf),
    Serialization(serde_json::Error),
}

impl std::fmt::Display for ReviewError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, source } => write!(formatter, "{}: {source}", path.display()),
            Self::InvalidJson { path, source } => {
                write!(formatter, "cannot parse {}: {source}", path.display())
            }
            Self::UnsupportedVersion { path, version } => write!(
                formatter,
                "{} uses unsupported review format version {version}",
                path.display()
            ),
            Self::RepositoryMismatch { path } => write!(
                formatter,
                "{} belongs to a different repository; original data was preserved",
                path.display()
            ),
            Self::MissingHomeDirectory => {
                formatter.write_str("cannot determine the user's data directory")
            }
            Self::InvalidRepositoryPath(path) => {
                write!(
                    formatter,
                    "cannot resolve repository path {}",
                    path.display()
                )
            }
            Self::StorageInsideRepository { path, repository } => write!(
                formatter,
                "review storage {} resolves inside repository {}; refusing to modify the working tree",
                path.display(),
                repository.display()
            ),
            Self::StorageSymlink(path) => write!(
                formatter,
                "review storage {} is a symbolic link; refusing to follow it",
                path.display()
            ),
            Self::Serialization(source) => write!(formatter, "cannot serialize reviews: {source}"),
        }
    }
}

impl std::error::Error for ReviewError {}

#[derive(Clone, Debug)]
pub struct ReviewStore {
    path: PathBuf,
    repository_root: PathBuf,
    pub document: ReviewDocument,
}

impl ReviewStore {
    pub fn open(repository_root: &Path) -> Result<Self, ReviewError> {
        let data_home = data_home()?;
        Self::open_at(repository_root, &data_home)
    }

    pub fn open_at(repository_root: &Path, data_home: &Path) -> Result<Self, ReviewError> {
        let canonical_root = repository_root
            .canonicalize()
            .map_err(|_| ReviewError::InvalidRepositoryPath(repository_root.to_path_buf()))?;
        let id = repository_id(&canonical_root)?;
        let path = data_home
            .join("miru")
            .join("reviews")
            .join(&id)
            .join("review.json");
        let resolved_storage_path =
            canonicalize_or_project(&path).map_err(|source| ReviewError::Io {
                path: path.clone(),
                source,
            })?;
        if resolved_storage_path.starts_with(&canonical_root) {
            return Err(ReviewError::StorageInsideRepository {
                path: resolved_storage_path,
                repository: canonical_root,
            });
        }
        if fs::symlink_metadata(&path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
            return Err(ReviewError::StorageSymlink(path));
        }
        if let Some(parent) = path.parent()
            && fs::symlink_metadata(parent).is_ok_and(|metadata| metadata.file_type().is_symlink())
        {
            return Err(ReviewError::StorageSymlink(parent.to_path_buf()));
        }

        let document = if path.exists() {
            let bytes = fs::read(&path).map_err(|source| ReviewError::Io {
                path: path.clone(),
                source,
            })?;
            let document: ReviewDocument =
                serde_json::from_slice(&bytes).map_err(|source| ReviewError::InvalidJson {
                    path: path.clone(),
                    source,
                })?;
            if document.version != REVIEW_VERSION {
                return Err(ReviewError::UnsupportedVersion {
                    path,
                    version: document.version,
                });
            }
            if document.repository_id != id {
                return Err(ReviewError::RepositoryMismatch { path });
            }
            document
        } else {
            ReviewDocument {
                version: REVIEW_VERSION,
                repository_id: id,
                repository_path: canonical_root.to_string_lossy().into_owned(),
                comments: Vec::new(),
            }
        };

        Ok(Self {
            path,
            repository_root: canonical_root,
            document,
        })
    }

    pub fn save(&self) -> Result<(), ReviewError> {
        let resolved_storage_path =
            canonicalize_or_project(&self.path).map_err(|source| ReviewError::Io {
                path: self.path.clone(),
                source,
            })?;
        if resolved_storage_path.starts_with(&self.repository_root) {
            return Err(ReviewError::StorageInsideRepository {
                path: resolved_storage_path,
                repository: self.repository_root.clone(),
            });
        }
        if fs::symlink_metadata(&self.path).is_ok_and(|metadata| metadata.file_type().is_symlink())
        {
            return Err(ReviewError::StorageSymlink(self.path.clone()));
        }
        let parent = self.path.parent().ok_or_else(|| ReviewError::Io {
            path: self.path.clone(),
            source: std::io::Error::other("review file has no parent directory"),
        })?;
        if fs::symlink_metadata(parent).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
            return Err(ReviewError::StorageSymlink(parent.to_path_buf()));
        }
        fs::create_dir_all(parent).map_err(|source| ReviewError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
        if fs::symlink_metadata(parent).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
            return Err(ReviewError::StorageSymlink(parent.to_path_buf()));
        }
        fs::set_permissions(parent, Permissions::from_mode(0o700)).map_err(|source| {
            ReviewError::Io {
                path: parent.to_path_buf(),
                source,
            }
        })?;

        let bytes =
            serde_json::to_vec_pretty(&self.document).map_err(ReviewError::Serialization)?;
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let temporary = parent.join(format!(".review-{}-{nonce}.tmp", std::process::id()));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)
            .map_err(|source| ReviewError::Io {
                path: temporary.clone(),
                source,
            })?;
        let write_result = file
            .write_all(&bytes)
            .and_then(|()| file.sync_all())
            .map_err(|source| ReviewError::Io {
                path: temporary.clone(),
                source,
            });
        drop(file);
        if let Err(error) = write_result {
            let _ = fs::remove_file(&temporary);
            return Err(error);
        }

        if let Err(source) = fs::rename(&temporary, &self.path) {
            let _ = fs::remove_file(&temporary);
            return Err(ReviewError::Io {
                path: self.path.clone(),
                source,
            });
        }
        if let Ok(directory) = File::open(parent) {
            let _ = directory.sync_all();
        }
        Ok(())
    }
}

fn canonicalize_or_project(path: &Path) -> std::io::Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    if let Ok(canonical) = absolute.canonicalize() {
        return Ok(canonical);
    }

    let mut ancestor = absolute.as_path();
    let mut missing = Vec::new();
    let canonical_ancestor = loop {
        if let Ok(metadata) = fs::symlink_metadata(ancestor)
            && metadata.file_type().is_symlink()
        {
            let target = fs::read_link(ancestor)?;
            let target = if target.is_absolute() {
                target
            } else {
                ancestor
                    .parent()
                    .unwrap_or_else(|| Path::new("/"))
                    .join(target)
            };
            let mut projected = target.canonicalize()?;
            append_missing_components(&mut projected, &missing);
            return Ok(projected);
        }
        match ancestor.canonicalize() {
            Ok(canonical) => break canonical,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        let Some(name) = ancestor.file_name() else {
            return Err(std::io::Error::other(
                "no existing ancestor for review storage path",
            ));
        };
        missing.push(name.to_os_string());
        let Some(parent) = ancestor.parent() else {
            return Err(std::io::Error::other(
                "no existing ancestor for review storage path",
            ));
        };
        ancestor = parent;
    };

    let mut projected = canonical_ancestor;
    append_missing_components(&mut projected, &missing);
    Ok(projected)
}

fn append_missing_components(path: &mut PathBuf, missing: &[std::ffi::OsString]) {
    for component in missing.iter().rev() {
        match component.to_str() {
            Some(".") => {}
            Some("..") => {
                path.pop();
            }
            _ => path.push(component),
        }
    }
}

pub fn repository_id(repository_root: &Path) -> Result<String, ReviewError> {
    let canonical_root = repository_root
        .canonicalize()
        .map_err(|_| ReviewError::InvalidRepositoryPath(repository_root.to_path_buf()))?;
    let mut digest = Sha256::new();
    digest.update(canonical_root.as_os_str().as_encoded_bytes());
    let bytes = digest.finalize();
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn data_home() -> Result<PathBuf, ReviewError> {
    if let Some(path) = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from)
        && path.is_absolute()
    {
        return Ok(path);
    }
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join(".local/share"))
        .ok_or(ReviewError::MissingHomeDirectory)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{ReviewStore, repository_id};
    use crate::review::{DiffSide, ReviewComment};

    struct TempTree(PathBuf);

    impl TempTree {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path =
                std::env::temp_dir().join(format!("miru-review-{}-{nonce}", std::process::id()));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempTree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn comment() -> ReviewComment {
        ReviewComment {
            id: "one".into(),
            file: "src/lib.rs".into(),
            start_line: 7,
            end_line: 8,
            side: DiffSide::New,
            body: "Please check this".into(),
            resolved: false,
            stale: false,
            anchor_hash: "abc".into(),
            anchor_line_count: 2,
            created_at: 1,
            updated_at: 2,
        }
    }

    #[test]
    fn repository_identity_distinguishes_same_named_directories() {
        let temp = TempTree::new();
        let first = temp.0.join("one/project");
        let second = temp.0.join("two/project");
        fs::create_dir_all(&first).unwrap();
        fs::create_dir_all(&second).unwrap();
        assert_ne!(
            repository_id(&first).unwrap(),
            repository_id(&second).unwrap()
        );
    }

    #[test]
    fn saved_reviews_load_back_from_versioned_json() {
        let temp = TempTree::new();
        let repository = temp.0.join("repo");
        let data_home = temp.0.join("data");
        fs::create_dir_all(&repository).unwrap();
        let mut store = ReviewStore::open_at(&repository, &data_home).unwrap();
        store.document.comments.push(comment());
        store.save().unwrap();
        let loaded = ReviewStore::open_at(&repository, &data_home).unwrap();
        assert_eq!(loaded.document.version, 1);
        assert_eq!(loaded.document.comments, vec![comment()]);
    }

    #[test]
    fn malformed_review_data_is_reported_without_overwriting_the_file() {
        let temp = TempTree::new();
        let repository = temp.0.join("repo");
        let data_home = temp.0.join("data");
        fs::create_dir_all(&repository).unwrap();
        let id = repository_id(&repository).unwrap();
        let path = data_home.join("miru/reviews").join(id).join("review.json");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"{").unwrap();
        assert!(ReviewStore::open_at(&repository, &data_home).is_err());
        assert_eq!(fs::read(path).unwrap(), b"{");
    }

    #[test]
    fn refuses_to_put_review_metadata_inside_the_repository() {
        let temp = TempTree::new();
        let repository = temp.0.join("repo");
        fs::create_dir_all(&repository).unwrap();
        assert!(ReviewStore::open_at(&repository, &repository).is_err());
        assert_eq!(fs::read_dir(repository).unwrap().count(), 0);
    }

    #[test]
    fn refuses_dangling_storage_symlinks_into_the_repository() {
        let temp = TempTree::new();
        let repository = temp.0.join("repo");
        let data_home = temp.0.join("data/miru/reviews");
        fs::create_dir_all(&repository).unwrap();
        fs::create_dir_all(&data_home).unwrap();
        let id = repository_id(&repository).unwrap();
        std::os::unix::fs::symlink(repository.join("review-data"), data_home.join(id)).unwrap();
        assert!(ReviewStore::open_at(&repository, &temp.0.join("data")).is_err());
        assert_eq!(fs::read_dir(repository).unwrap().count(), 0);
    }
}
