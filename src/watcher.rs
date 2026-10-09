use std::path::Path;
use std::sync::mpsc::{self, Receiver};

use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};

pub struct RepositoryWatcher {
    pub events: Receiver<()>,
    _watcher: RecommendedWatcher,
}

impl RepositoryWatcher {
    pub fn new(root: &Path, git_dir: &Path) -> notify::Result<Self> {
        let (sender, events) = mpsc::channel();
        let git_dir = git_dir.to_path_buf();
        let watch_git_dir = !git_dir.starts_with(root);
        let filter_git_dir = git_dir.clone();
        let mut watcher = notify::recommended_watcher(move |event: notify::Result<Event>| {
            let Ok(event) = event else {
                return;
            };
            let index_path = filter_git_dir.join("index");
            let head_path = filter_git_dir.join("HEAD");
            let relevant = event.paths.is_empty()
                || event.paths.iter().any(|path| {
                    !path.starts_with(&filter_git_dir) || path == &index_path || path == &head_path
                });
            if relevant {
                let _ = sender.send(());
            }
        })?;
        watcher.watch(root, RecursiveMode::Recursive)?;
        if watch_git_dir {
            watcher.watch(&git_dir, RecursiveMode::NonRecursive)?;
        }
        Ok(Self {
            events,
            _watcher: watcher,
        })
    }
}
