use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io::IsTerminal;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gtk::prelude::*;

use crate::git::{DiffDocument, Repository};
use crate::review::{
    DiffSide, ReviewComment, ReviewStore, anchor_for_selection, export_markdown,
    reconcile_file_comments,
};
use crate::theme::{ADDED_FOREGROUND, REMOVED_FOREGROUND};
use crate::ui::{DiffView, ReviewPanel, build_file_list, install_shortcuts, make_file_row};
use crate::watcher::RepositoryWatcher;

pub struct AppState {
    pub window: gtk::ApplicationWindow,
    application: gtk::Application,
    repository: Repository,
    sidebar: gtk::Box,
    sidebar_toggle_button: gtk::Button,
    review_toggle_button: gtk::Button,
    file_list: gtk::ListBox,
    diff_view: DiffView,
    review_panel: ReviewPanel,
    count_label: gtk::Label,
    status_label: gtk::Label,
    review_count_label: gtk::Label,
    position_label: gtk::Label,
    hunk_label: gtk::Label,
    files: Vec<crate::git::ChangedFile>,
    selected_path: Option<PathBuf>,
    document: DiffDocument,
    current_hunk: usize,
    pending_key: Option<char>,
    focused_diff_line: Option<usize>,
    selection_anchor: Option<usize>,
    pending_comment: Option<CommentDraft>,
    review_store: Option<ReviewStore>,
    review_load_error: Option<String>,
    review_save_error: Option<String>,
    watcher: Option<RepositoryWatcher>,
    last_fs_event: Option<Instant>,
}

pub fn build(application: &gtk::Application, repository: Repository) {
    crate::theme::install();

    let window = gtk::ApplicationWindow::builder()
        .application(application)
        .title("Zako")
        .default_width(1120)
        .default_height(740)
        .build();

    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.add_css_class("zako-root");

    let header = gtk::HeaderBar::new();
    header.add_css_class("zako-header");
    let sidebar_toggle_button = gtk::Button::with_label("‹");
    sidebar_toggle_button.add_css_class("zako-sidebar-toggle");
    sidebar_toggle_button.set_tooltip_text(Some("Hide changed files (Tab)"));
    header.pack_start(&sidebar_toggle_button);
    let review_toggle_button = gtk::Button::with_label("Review");
    review_toggle_button.add_css_class("zako-review-toggle");
    review_toggle_button.set_tooltip_text(Some("Toggle review comments (C)"));
    header.pack_start(&review_toggle_button);
    let repo_name = repository
        .root()
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("repository");
    window.set_title(Some(&format!("{repo_name} — Zako")));
    let brand = gtk::Label::new(Some("zako"));
    brand.add_css_class("zako-brand");
    let separator = gtk::Label::new(Some("/"));
    separator.add_css_class("zako-header-separator");
    let repository_label = gtk::Label::new(Some(repo_name));
    repository_label.add_css_class("zako-repo-name");
    repository_label.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
    header.pack_start(&brand);
    header.pack_start(&separator);
    header.pack_start(&repository_label);
    let count_label = gtk::Label::new(Some("0 FILES"));
    count_label.add_css_class("zako-file-count");
    header.pack_end(&count_label);

    let (sidebar, file_list) = build_file_list();
    let diff_view = DiffView::new();
    let review_panel = ReviewPanel::new();
    review_panel.container.set_visible(false);
    let review_split = gtk::Paned::new(gtk::Orientation::Horizontal);
    review_split.add_css_class("zako-paned");
    review_split.set_start_child(Some(&diff_view.container));
    review_split.set_end_child(Some(&review_panel.container));
    review_split.set_position(760);
    review_split.set_resize_start_child(true);
    review_split.set_shrink_start_child(false);
    review_split.set_wide_handle(false);
    let pane = gtk::Paned::new(gtk::Orientation::Horizontal);
    pane.add_css_class("zako-paned");
    pane.set_start_child(Some(&sidebar));
    pane.set_end_child(Some(&review_split));
    pane.set_position(270);
    pane.set_resize_start_child(false);
    pane.set_shrink_start_child(false);
    pane.set_wide_handle(false);
    pane.set_vexpand(true);

    let status_line = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    status_line.add_css_class("zako-status-line");
    let status_label = gtk::Label::new(Some("WORKING TREE CLEAN"));
    status_label.set_xalign(0.0);
    status_label.set_hexpand(true);
    status_label.add_css_class("zako-status");
    let hunk_label = gtk::Label::new(None);
    hunk_label.set_xalign(1.0);
    hunk_label.add_css_class("zako-hunk-status");
    let review_count_label = gtk::Label::new(None);
    review_count_label.set_xalign(1.0);
    review_count_label.add_css_class("zako-review-status");
    let position_label = gtk::Label::new(None);
    position_label.set_xalign(1.0);
    position_label.add_css_class("zako-hunk-status");
    status_line.append(&status_label);
    status_line.append(&review_count_label);
    status_line.append(&position_label);
    status_line.append(&hunk_label);

    root.append(&pane);
    root.append(&status_line);
    window.set_titlebar(Some(&header));
    window.set_child(Some(&root));

    let watcher = match RepositoryWatcher::new(repository.root(), repository.git_dir()) {
        Ok(watcher) => Some(watcher),
        Err(error) => {
            eprintln!("zako: filesystem watching unavailable: {error}");
            None
        }
    };

    let (review_store, review_load_error) = match ReviewStore::open(repository.root()) {
        Ok(store) => (Some(store), None),
        Err(error) => {
            eprintln!("zako: review data unavailable: {error}");
            (None, Some(error.to_string()))
        }
    };
    let state = Rc::new(RefCell::new(AppState {
        window: window.clone(),
        application: application.clone(),
        repository,
        sidebar,
        sidebar_toggle_button: sidebar_toggle_button.clone(),
        review_toggle_button: review_toggle_button.clone(),
        file_list: file_list.clone(),
        diff_view,
        review_panel,
        count_label,
        status_label,
        review_count_label,
        position_label,
        hunk_label,
        files: Vec::new(),
        selected_path: None,
        document: DiffDocument::default(),
        current_hunk: 0,
        pending_key: None,
        focused_diff_line: None,
        selection_anchor: None,
        pending_comment: None,
        review_store,
        review_load_error,
        review_save_error: None,
        watcher,
        last_fs_event: None,
    }));

    let callback_state = state.clone();
    file_list.connect_row_selected(move |_, row| {
        let Some(row) = row else { return };
        if let Ok(mut state) = callback_state.try_borrow_mut() {
            state.select_index(row.index() as usize, &callback_state);
        }
    });

    let callback_state = state.clone();
    sidebar_toggle_button.connect_clicked(move |_| {
        if let Ok(mut state) = callback_state.try_borrow_mut() {
            state.toggle_sidebar();
        }
    });

    let callback_state = state.clone();
    review_toggle_button.connect_clicked(move |_| {
        if let Ok(mut state) = callback_state.try_borrow_mut() {
            state.toggle_review_panel();
        }
    });

    let callback_state = state.clone();
    let copy_button = state.borrow().review_panel.copy_button.clone();
    copy_button.connect_clicked(move |_| {
        if let Ok(mut state) = callback_state.try_borrow_mut() {
            state.copy_review_to_clipboard();
        }
    });

    let callback_state = state.clone();
    let diff_buffer = state.borrow().diff_view.buffer().clone();
    diff_buffer.connect_mark_set(move |_, iter, mark| {
        if mark.name().as_deref() != Some("insert") {
            return;
        }
        if let Ok(mut state) = callback_state.try_borrow_mut() {
            state.focus_buffer_line(iter.line() as usize);
        }
    });

    install_shortcuts(&window, &state);
    install_refresh_poll(&state);
    let initial_state = state.clone();
    state.borrow_mut().refresh(&initial_state);
    window.present();
}

impl AppState {
    pub fn handle_key(&mut self, key: char, shared: &Rc<RefCell<AppState>>) -> bool {
        if let Some(previous) = self.pending_key.take() {
            match (previous, key) {
                ('g', 'g') => {
                    self.diff_view.scroll_to_top();
                    if let Some(line) = first_changed_line(&self.document) {
                        self.focus_diff_line(line);
                    }
                    return true;
                }
                (']', 'f') => {
                    self.move_file(1, shared);
                    return true;
                }
                ('[', 'f') => {
                    self.move_file(-1, shared);
                    return true;
                }
                (']', 'h') => {
                    self.move_hunk(1);
                    return true;
                }
                ('[', 'h') => {
                    self.move_hunk(-1);
                    return true;
                }
                (']', 'c') => {
                    self.move_comment(1, shared);
                    return true;
                }
                ('[', 'c') => {
                    self.move_comment(-1, shared);
                    return true;
                }
                (_, next) => {
                    self.pending_key = match next {
                        'g' | '[' | ']' => Some(next),
                        _ => None,
                    }
                }
            }
        } else if matches!(key, 'g' | '[' | ']') {
            self.pending_key = Some(key);
            return true;
        }

        match key {
            'j' if self.selection_anchor.is_some() => self.move_selection_focus(1),
            'k' if self.selection_anchor.is_some() => self.move_selection_focus(-1),
            'j' => self.move_focused_line(1),
            'k' => self.move_focused_line(-1),
            'G' => {
                self.diff_view.scroll_to_bottom();
                if let Some(line) = last_changed_line(&self.document) {
                    self.focus_diff_line(line);
                }
            }
            'r' => self.refresh(shared),
            '?' => self.show_shortcuts(),
            'C' => self.toggle_review_panel(),
            'v' => self.start_line_selection(),
            'c' => self.begin_or_edit_comment(),
            'e' => self.open_editor(),
            '\n' | '\r' => self.edit_comment_on_current_line(),
            '\u{1b}' if self.pending_comment.is_some() => self.cancel_pending_comment(shared),
            '\u{1b}' => self.cancel_line_selection(),
            'q' => self.application.quit(),
            _ => return false,
        }
        true
    }

    pub fn toggle_sidebar(&mut self) {
        let visible = !self.sidebar.is_visible();
        self.sidebar.set_visible(visible);
        self.sidebar_toggle_button
            .set_label(if visible { "‹" } else { "›" });
        self.sidebar_toggle_button
            .set_tooltip_text(Some(if visible {
                "Hide changed files (Tab)"
            } else {
                "Show changed files (Tab)"
            }));
    }

    fn toggle_review_panel(&mut self) {
        let visible = !self.review_panel.container.is_visible();
        self.review_panel.container.set_visible(visible);
        self.review_toggle_button
            .set_label(if visible { "Review ·" } else { "Review" });
    }

    fn move_file(&mut self, delta: isize, shared: &Rc<RefCell<AppState>>) {
        if self.files.is_empty() {
            return;
        }
        let current = self
            .selected_path
            .as_ref()
            .and_then(|path| self.files.iter().position(|file| &file.path == path))
            .unwrap_or(0) as isize;
        let next = (current + delta).clamp(0, self.files.len() as isize - 1) as usize;
        self.select_index(next, shared);
        self.file_list
            .select_row(self.file_list.row_at_index(next as i32).as_ref());
    }

    fn move_hunk(&mut self, delta: isize) {
        if self.document.hunk_lines.is_empty() {
            return;
        }
        let last = self.document.hunk_lines.len() - 1;
        self.current_hunk = (self.current_hunk as isize + delta).clamp(0, last as isize) as usize;
        if let Some(line) = self.document.hunk_lines.get(self.current_hunk).copied() {
            self.focused_diff_line = Some(line);
            self.diff_view.focus_line(line);
        }
        self.update_status();
    }

    fn select_index(&mut self, index: usize, shared: &Rc<RefCell<AppState>>) {
        let Some(file) = self.files.get(index) else {
            return;
        };
        self.selected_path = Some(file.path.clone());
        self.current_hunk = 0;
        self.focused_diff_line = None;
        self.selection_anchor = None;
        self.load_selected();
        self.rebuild_review_panel(shared);
    }

    fn refresh(&mut self, shared: &Rc<RefCell<AppState>>) {
        match self.repository.status() {
            Ok(snapshot) => {
                let selected_before = self.selected_path.clone();
                let changed_files = self.files != snapshot.files;
                self.files = snapshot.files;
                let changed_paths: std::collections::HashSet<String> = self
                    .files
                    .iter()
                    .map(|file| file.path.to_string_lossy().into_owned())
                    .collect();
                let stale_changed = self.mark_removed_comments_stale(&changed_paths);
                self.selected_path = selected_before
                    .filter(|path| self.files.iter().any(|file| &file.path == path))
                    .or_else(|| self.files.first().map(|file| file.path.clone()));

                if changed_files {
                    self.rebuild_file_list();
                }
                self.sync_selected_row();
                self.status_label.remove_css_class("zako-error");
                if stale_changed {
                    self.persist_reviews();
                }
                self.load_selected();
                self.count_label
                    .set_text(&format!("{} FILES", self.files.len()));
                self.rebuild_review_panel(shared);
            }
            Err(error) => {
                self.status_label.add_css_class("zako-error");
                self.status_label.set_text(&error.to_string());
                self.hunk_label.set_text("");
            }
        }
    }

    fn rebuild_file_list(&self) {
        while let Some(child) = self.file_list.first_child() {
            self.file_list.remove(&child);
        }
        for file in &self.files {
            self.file_list.append(&make_file_row(file));
        }
    }

    fn sync_selected_row(&self) {
        let index = self
            .selected_path
            .as_ref()
            .and_then(|path| self.files.iter().position(|file| &file.path == path));
        if let Some(index) = index {
            self.file_list
                .select_row(self.file_list.row_at_index(index as i32).as_ref());
        } else {
            self.file_list.unselect_all();
        }
    }

    fn load_selected(&mut self) {
        let Some(path) = self.selected_path.clone() else {
            self.document = DiffDocument::default();
            self.focused_diff_line = None;
            self.diff_view.show_document(None, &self.document, &[]);
            self.update_status();
            return;
        };
        let Some(file) = self.files.iter().find(|file| file.path == path) else {
            return;
        };
        match self.repository.diff(file) {
            Ok(document) => {
                self.status_label.remove_css_class("zako-error");
                let before = self.comments_for_file(&path);
                let file_key = path.to_string_lossy().into_owned();
                if let Some(store) = &mut self.review_store {
                    reconcile_file_comments(&mut store.document.comments, &file_key, &document);
                    let now = timestamp();
                    for comment in store
                        .document
                        .comments
                        .iter_mut()
                        .filter(|comment| comment.file == file_key)
                    {
                        if before.iter().find(|old| old.id == comment.id) != Some(comment) {
                            comment.updated_at = now;
                        }
                    }
                }
                let comments_changed = before != self.comments_for_file(&path);
                let document_changed = self.document != document;
                if document_changed {
                    let previous_focus = self
                        .focused_diff_line
                        .and_then(|index| diff_line_anchor(&self.document, index));
                    let was_selecting = self.selection_anchor.is_some();
                    let previous_selection = self
                        .selection_anchor
                        .and_then(|index| diff_line_anchor(&self.document, index));
                    let had_focus_anchor = previous_focus.is_some();
                    self.document = document;
                    self.current_hunk = self
                        .current_hunk
                        .min(self.document.hunk_lines.len().saturating_sub(1));
                    self.focused_diff_line = previous_focus
                        .and_then(|anchor| find_anchored_line(&self.document, &anchor))
                        .or_else(|| first_changed_line(&self.document));
                    self.selection_anchor = if was_selecting && had_focus_anchor {
                        previous_selection
                            .and_then(|anchor| find_anchored_line(&self.document, &anchor))
                    } else {
                        None
                    };
                }
                if document_changed
                    || comments_changed
                    || self.diff_view.heading.text().as_str() != path.display().to_string()
                {
                    let comments = self.comments_for_file(&path);
                    self.diff_view
                        .show_document(Some(&path), &self.document, &comments);
                    if !self.is_composing_comment()
                        && let Some(line) = self.focused_diff_line
                    {
                        self.diff_view.focus_line(line);
                    }
                }
                if comments_changed {
                    self.persist_reviews();
                }
                self.update_status();
            }
            Err(error) => {
                self.diff_view.show_error(&error.to_string());
                self.status_label.add_css_class("zako-error");
                self.status_label.set_text(&error.to_string());
                self.hunk_label.set_text("");
            }
        }
    }

    fn comments(&self) -> &[ReviewComment] {
        self.review_store
            .as_ref()
            .map_or(&[], |store| store.document.comments.as_slice())
    }

    fn comments_for_file(&self, path: &std::path::Path) -> Vec<ReviewComment> {
        let path = path.to_string_lossy();
        self.comments()
            .iter()
            .filter(|comment| comment.file == path)
            .cloned()
            .collect()
    }

    fn mark_removed_comments_stale(
        &mut self,
        changed_paths: &std::collections::HashSet<String>,
    ) -> bool {
        let Some(store) = &mut self.review_store else {
            return false;
        };
        let now = timestamp();
        let mut changed = false;
        for comment in &mut store.document.comments {
            if !changed_paths.contains(&comment.file) && !comment.stale {
                comment.stale = true;
                comment.updated_at = now;
                changed = true;
            }
        }
        changed
    }

    fn persist_reviews(&mut self) -> bool {
        let Some(store) = &self.review_store else {
            return false;
        };
        match store.save() {
            Ok(()) => {
                self.review_save_error = None;
                true
            }
            Err(error) => {
                self.review_save_error = Some(error.to_string());
                false
            }
        }
    }

    fn rebuild_review_panel(&self, shared: &Rc<RefCell<AppState>>) {
        self.review_panel.clear();
        if let Some(error) = &self.review_load_error {
            self.review_panel
                .append_empty(&format!("Review data is unavailable: {error}"));
            self.review_count_label.set_text("REVIEW DATA ERROR");
            self.review_count_label.set_tooltip_text(Some(error));
            return;
        }
        let mut by_file: BTreeMap<&str, Vec<&ReviewComment>> = BTreeMap::new();
        for comment in self.comments() {
            by_file.entry(&comment.file).or_default().push(comment);
        }
        if by_file.is_empty() {
            self.review_panel.append_empty("No review comments yet.");
        }
        for (file, mut comments) in by_file {
            comments.sort_by_key(|comment| (comment.start_line, comment.end_line));
            self.review_panel.append_file_heading(file);
            for comment in comments {
                let row = self.review_panel.append_comment(comment);
                let id = comment.id.clone();
                let state_for_jump = shared.clone();
                let jump_id = id.clone();
                row.jump.connect_clicked(move |_| {
                    if let Ok(mut app) = state_for_jump.try_borrow_mut() {
                        app.jump_to_comment(&jump_id, &state_for_jump);
                    }
                });

                let state_for_edit = shared.clone();
                let edit_id = id.clone();
                row.edit.connect_clicked(move |_| {
                    begin_edit_comment(&state_for_edit, &edit_id);
                });

                let state_for_resolve = shared.clone();
                let resolve_id = id.clone();
                row.resolve.connect_clicked(move |_| {
                    if let Ok(mut app) = state_for_resolve.try_borrow_mut() {
                        app.toggle_comment_resolved(&resolve_id, &state_for_resolve);
                    }
                });

                let state_for_delete = shared.clone();
                row.delete.connect_clicked(move |_| {
                    if let Ok(mut app) = state_for_delete.try_borrow_mut() {
                        app.delete_comment(&id, &state_for_delete);
                    }
                });
            }
        }

        let unresolved = self
            .comments()
            .iter()
            .filter(|comment| !comment.resolved)
            .count();
        let resolved = self.comments().len().saturating_sub(unresolved);
        self.review_count_label
            .set_text(&format!("{unresolved} OPEN · {resolved} RESOLVED"));
        self.review_count_label.set_tooltip_text(None);
    }

    fn toggle_comment_resolved(&mut self, id: &str, shared: &Rc<RefCell<AppState>>) {
        let Some(store) = &mut self.review_store else {
            return;
        };
        let Some(comment) = store
            .document
            .comments
            .iter_mut()
            .find(|comment| comment.id == id)
        else {
            return;
        };
        comment.resolved = !comment.resolved;
        comment.updated_at = timestamp();
        self.persist_reviews();
        self.refresh_review_views(shared);
    }

    fn delete_comment(&mut self, id: &str, shared: &Rc<RefCell<AppState>>) {
        let Some(store) = &mut self.review_store else {
            return;
        };
        let before = store.document.comments.len();
        store.document.comments.retain(|comment| comment.id != id);
        if store.document.comments.len() == before {
            return;
        }
        self.persist_reviews();
        self.refresh_review_views(shared);
    }

    fn save_comment(
        &mut self,
        id: Option<&str>,
        body: String,
        anchor: Option<CommentTarget>,
        shared: &Rc<RefCell<AppState>>,
    ) {
        let Some(store) = &mut self.review_store else {
            return;
        };
        let now = timestamp();
        if let Some(id) = id {
            if let Some(comment) = store
                .document
                .comments
                .iter_mut()
                .find(|comment| comment.id == id)
            {
                comment.body = body;
                comment.updated_at = now;
            }
        } else if let Some(target) = anchor {
            store.document.comments.push(ReviewComment {
                id: new_comment_id(),
                file: target.file,
                start_line: target.start_line,
                end_line: target.end_line,
                side: target.side,
                body,
                resolved: false,
                stale: false,
                anchor_hash: target.anchor_hash,
                anchor_line_count: target.anchor_line_count,
                created_at: now,
                updated_at: now,
            });
        }
        self.persist_reviews();
        self.refresh_review_views(shared);
    }

    pub fn save_pending_comment(&mut self, shared: &Rc<RefCell<AppState>>) {
        let body = self.diff_view.inline_comment_body().unwrap_or_default();
        if body.is_empty() {
            self.status_label.set_text("Write a comment before saving.");
            self.diff_view.focus_inline_comment();
            return;
        }
        let Some(draft) = self.pending_comment.take() else {
            return;
        };
        self.diff_view.end_inline_comment();
        self.selection_anchor = None;
        self.save_comment(draft.id.as_deref(), body, draft.target, shared);
    }

    pub fn cancel_pending_comment(&mut self, shared: &Rc<RefCell<AppState>>) {
        self.pending_comment = None;
        self.diff_view.end_inline_comment();
        self.refresh_review_views(shared);
    }

    pub fn is_composing_comment(&self) -> bool {
        self.diff_view.is_editing_comment()
    }

    pub fn backspace_empty_comment(&mut self, shared: &Rc<RefCell<AppState>>) -> bool {
        if !self
            .diff_view
            .inline_comment_body()
            .unwrap_or_default()
            .is_empty()
        {
            return false;
        }
        let Some(id) = self
            .pending_comment
            .as_ref()
            .and_then(|draft| draft.id.clone())
        else {
            return false;
        };

        self.pending_comment = None;
        self.diff_view.end_inline_comment();
        self.delete_comment(&id, shared);
        self.diff_view.source_view.grab_focus();
        self.status_label.set_text("Review comment deleted.");
        true
    }

    fn refresh_review_views(&self, shared: &Rc<RefCell<AppState>>) {
        self.rebuild_review_panel(shared);
        if let Some(path) = &self.selected_path {
            let comments = self.comments_for_file(path);
            self.diff_view
                .show_document_preserving_scroll(Some(path), &self.document, &comments);
            if !self.is_composing_comment()
                && let Some(line) = self.focused_diff_line
            {
                self.diff_view.focus_line(line);
            }
        }
        self.update_status();
    }

    fn focus_diff_line(&mut self, line: usize) {
        if self.document.lines.get(line).is_none() {
            return;
        }
        self.focused_diff_line = Some(line);
        if let Some(anchor) = self.selection_anchor {
            self.diff_view.select_display_range(anchor, line);
        } else {
            self.diff_view.focus_line(line);
        }
        self.update_status();
    }

    fn focus_buffer_line(&mut self, buffer_line: usize) {
        if self.pending_comment.is_some() {
            return;
        }
        let Some(line) = self.diff_view.document_line_at_buffer_line(buffer_line) else {
            return;
        };
        if self.focused_diff_line == Some(line) && self.selection_anchor.is_none() {
            return;
        }
        self.focused_diff_line = Some(line);
        self.selection_anchor = None;
        self.diff_view.highlight_document_line(line);
        self.update_status();
    }

    fn start_line_selection(&mut self) {
        let focus = self
            .focused_diff_line
            .filter(|index| self.document.lines.get(*index).is_some_and(is_changed_line))
            .or_else(|| {
                self.focused_diff_line.and_then(|index| {
                    (index..self.document.lines.len())
                        .find(|line| is_changed_line(&self.document.lines[*line]))
                })
            })
            .or_else(|| {
                self.focused_diff_line.and_then(|index| {
                    (0..index.min(self.document.lines.len()))
                        .rev()
                        .find(|line| is_changed_line(&self.document.lines[*line]))
                })
            })
            .or_else(|| first_changed_line(&self.document));
        let Some(focus) = focus else {
            return;
        };
        self.focused_diff_line = Some(focus);
        self.selection_anchor = Some(focus);
        self.diff_view.select_display_range(focus, focus);
        self.update_status();
    }

    fn move_selection_focus(&mut self, delta: isize) {
        let (Some(anchor), Some(current)) = (self.selection_anchor, self.focused_diff_line) else {
            return;
        };
        let next = (current as isize + delta)
            .clamp(0, self.document.lines.len().saturating_sub(1) as isize)
            as usize;
        self.focused_diff_line = Some(next);
        self.diff_view.select_display_range(anchor, next);
        self.update_status();
    }

    fn move_focused_line(&mut self, delta: isize) {
        if self.document.lines.is_empty() {
            return;
        }
        let current = self.focused_diff_line.unwrap_or(if delta > 0 {
            0
        } else {
            self.document.lines.len() - 1
        });
        let next = (current as isize + delta)
            .clamp(0, self.document.lines.len().saturating_sub(1) as isize)
            as usize;
        self.focused_diff_line = Some(next);
        if let Some(anchor) = self.selection_anchor {
            self.diff_view.select_display_range(anchor, next);
        } else {
            self.diff_view.focus_line(next);
        }
        self.update_status();
    }

    fn cancel_line_selection(&mut self) {
        let focus = self.selection_anchor.take().or(self.focused_diff_line);
        if let Some(focus) = focus {
            self.focused_diff_line = Some(focus);
            self.diff_view.focus_line(focus);
        } else {
            self.diff_view.clear_selection();
        }
        self.update_status();
    }

    fn begin_new_comment(&mut self) {
        if self.review_store.is_none() {
            self.status_label.add_css_class("zako-error");
            self.status_label
                .set_text("Review storage is unavailable; comments cannot be saved.");
            return;
        }
        let Some(path) = self.selected_path.as_ref() else {
            self.status_label.set_text("Select a changed file first.");
            return;
        };
        let Some(focus) = self
            .focused_diff_line
            .or_else(|| first_changed_line(&self.document))
        else {
            self.status_label
                .set_text("Select an added or removed diff line first.");
            return;
        };
        self.focused_diff_line = Some(focus);
        let range = self
            .selection_anchor
            .map(|anchor| (anchor.min(focus), anchor.max(focus)))
            .or_else(|| self.diff_view.selected_display_range())
            .unwrap_or((focus, focus));
        let side_line = self.selection_anchor.unwrap_or(focus);
        let Some(line) = self.document.lines.get(side_line) else {
            return;
        };
        let side = match line.kind {
            crate::git::LineKind::Addition => DiffSide::New,
            crate::git::LineKind::Deletion => DiffSide::Old,
            _ => {
                self.status_label
                    .set_text("Comments can be attached to added or removed lines.");
                return;
            }
        };
        let Some(anchor) = anchor_for_selection(
            &self.document,
            side,
            range.0.min(range.1),
            range.0.max(range.1),
        ) else {
            self.status_label
                .set_text("Select a contiguous range of changed lines on one side.");
            return;
        };
        let target = CommentTarget {
            file: path.to_string_lossy().into_owned(),
            start_line: anchor.start_line,
            end_line: anchor.end_line,
            side: anchor.side,
            anchor_hash: anchor.anchor_hash,
            anchor_line_count: anchor.anchor_line_count,
        };
        self.pending_comment = Some(CommentDraft {
            id: None,
            target: Some(target.clone()),
        });
        self.diff_view
            .begin_inline_comment(None, target.side, target.start_line, None);
        let comments = self.comments_for_file(path);
        self.diff_view
            .show_document_preserving_scroll(Some(path), &self.document, &comments);
    }

    fn begin_or_edit_comment(&mut self) {
        if self.selection_anchor.is_none()
            && let Some(comment) = self.comment_on_current_line()
        {
            self.start_edit_comment(comment);
        } else {
            self.begin_new_comment();
        }
    }

    fn edit_comment_on_current_line(&mut self) {
        if let Some(comment) = self.comment_on_current_line() {
            self.start_edit_comment(comment);
        }
    }

    fn comment_on_current_line(&self) -> Option<ReviewComment> {
        let path = self.selected_path.as_ref()?.to_string_lossy();
        let index = self.focused_diff_line?;
        let line = self.document.lines.get(index)?;
        let (side, line_number) = match line.kind {
            crate::git::LineKind::Addition => (DiffSide::New, line.new_line),
            crate::git::LineKind::Deletion => (DiffSide::Old, line.old_line),
            _ => return None,
        };
        let line_number = line_number?;
        self.comments()
            .iter()
            .find(|comment| {
                comment.file == path
                    && !comment.stale
                    && comment.side == side
                    && line_number >= comment.start_line
                    && line_number <= comment.end_line
            })
            .cloned()
    }

    fn start_edit_comment(&mut self, comment: ReviewComment) {
        self.pending_comment = Some(CommentDraft {
            id: Some(comment.id.clone()),
            target: None,
        });
        let comments = self.comments_for_file(std::path::Path::new(&comment.file));
        self.diff_view.begin_inline_comment(
            Some(comment.id),
            comment.side,
            comment.start_line,
            Some(&comment.body),
        );
        self.diff_view.show_document_preserving_scroll(
            self.selected_path.as_deref(),
            &self.document,
            &comments,
        );
    }

    fn start_edit_comment_by_id(&mut self, id: &str, shared: &Rc<RefCell<AppState>>) {
        self.jump_to_comment(id, shared);
        if let Some(comment) = self
            .comments()
            .iter()
            .find(|comment| comment.id == id)
            .cloned()
        {
            self.start_edit_comment(comment);
        }
    }

    fn move_comment(&mut self, delta: isize, shared: &Rc<RefCell<AppState>>) {
        let Some(current_file) = self
            .selected_path
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned())
        else {
            return;
        };
        let mut unresolved: Vec<_> = self
            .comments()
            .iter()
            .filter(|comment| !comment.resolved)
            .collect();
        unresolved.sort_by_key(|comment| (&comment.file, comment.start_line, comment.end_line));
        if unresolved.is_empty() {
            return;
        }
        let current = self.focused_diff_line.and_then(|index| {
            let line = self.document.lines.get(index)?;
            let (side, number) = match line.kind {
                crate::git::LineKind::Addition => (DiffSide::New, line.new_line?),
                crate::git::LineKind::Deletion => (DiffSide::Old, line.old_line?),
                _ => return None,
            };
            unresolved.iter().position(|comment| {
                comment.file == current_file
                    && comment.side == side
                    && number >= comment.start_line
                    && number <= comment.end_line
            })
        });
        let index = current.unwrap_or(usize::MAX);
        let next = if delta > 0 {
            if index == usize::MAX {
                0
            } else {
                (index + 1).min(unresolved.len() - 1)
            }
        } else if index == usize::MAX {
            unresolved.len() - 1
        } else {
            index.saturating_sub(1)
        };
        let id = unresolved[next].id.clone();
        self.jump_to_comment(&id, shared);
    }

    fn jump_to_comment(&mut self, id: &str, shared: &Rc<RefCell<AppState>>) {
        let Some(mut comment) = self
            .comments()
            .iter()
            .find(|comment| comment.id == id)
            .cloned()
        else {
            return;
        };
        if let Some(index) = self
            .files
            .iter()
            .position(|file| file.path.to_string_lossy() == comment.file)
        {
            self.select_index(index, shared);
            self.file_list
                .select_row(self.file_list.row_at_index(index as i32).as_ref());
        } else {
            self.selected_path = Some(PathBuf::from(&comment.file));
            self.document = DiffDocument::default();
            self.focused_diff_line = None;
            let comments = self.comments_for_file(std::path::Path::new(&comment.file));
            self.diff_view
                .show_document(self.selected_path.as_deref(), &self.document, &comments);
        }
        if let Some(updated) = self.comments().iter().find(|current| current.id == id) {
            comment = updated.clone();
        }
        let target = self
            .document
            .lines
            .iter()
            .position(|line| match comment.side {
                DiffSide::Old => line.old_line.is_some_and(|number| {
                    number >= comment.start_line && number <= comment.end_line
                }),
                DiffSide::New => line.new_line.is_some_and(|number| {
                    number >= comment.start_line && number <= comment.end_line
                }),
            });
        if let Some(index) = target {
            self.focused_diff_line = Some(index);
            self.selection_anchor = None;
            self.diff_view.focus_line(index);
            self.update_status();
        } else {
            self.status_label
                .set_text("This review location is stale or no longer in the diff.");
        }
    }

    fn open_editor(&mut self) {
        let Some(path) = self.selected_path.as_ref() else {
            self.status_label.set_text("Select a changed file first.");
            return;
        };
        let Some(line) = self
            .focused_diff_line
            .and_then(|index| self.document.lines.get(index))
        else {
            self.status_label.set_text("Select a changed line first.");
            return;
        };
        let line_number = line.new_line.or(line.old_line).unwrap_or(1);
        let Some(editor) = std::env::var_os("EDITOR") else {
            self.status_label
                .set_text("Set $EDITOR to open files from Zako.");
            return;
        };
        let Some(mut command_parts) = shlex::split(&editor.to_string_lossy()) else {
            self.status_label.set_text("Could not parse $EDITOR.");
            return;
        };
        if command_parts.is_empty() {
            self.status_label.set_text("$EDITOR is empty.");
            return;
        }
        let executable = command_parts.remove(0);
        let name = std::path::Path::new(&executable)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("")
            .to_owned();
        let absolute_path = self.repository.root().join(path);
        let gui_editor = matches!(
            name.as_str(),
            "code" | "code-insiders" | "nvim-qt" | "gvim" | "emacsclient"
        );
        let mut editor_args: Vec<std::ffi::OsString> =
            command_parts.into_iter().map(Into::into).collect();
        if name == "code" || name == "code-insiders" {
            editor_args.push("--goto".into());
            editor_args.push(format!("{}:{line_number}", absolute_path.display()).into());
        } else {
            editor_args.push(format!("+{line_number}").into());
            editor_args.push(absolute_path.into_os_string());
        }

        let has_terminal = std::io::stdin().is_terminal()
            || std::io::stdout().is_terminal()
            || std::io::stderr().is_terminal();
        let result = if gui_editor || has_terminal {
            std::process::Command::new(executable)
                .args(editor_args)
                .spawn()
                .map(|_| ())
        } else {
            launch_editor_in_terminal(&executable, &editor_args).map(|_| ())
        };
        match result {
            Ok(_) => self
                .status_label
                .set_text("Opened current location in $EDITOR."),
            Err(error) => {
                self.status_label.add_css_class("zako-error");
                self.status_label
                    .set_text(&format!("Could not start $EDITOR: {error}"));
            }
        }
    }

    fn copy_review_to_clipboard(&mut self) {
        let Some(store) = &self.review_store else {
            self.status_label.add_css_class("zako-error");
            self.status_label.set_text("Review data is unavailable.");
            return;
        };
        let markdown = export_markdown(&store.document);
        gtk::prelude::WidgetExt::display(&self.window)
            .clipboard()
            .set_text(&markdown);
        self.status_label
            .set_text("Unresolved review copied to clipboard.");
    }

    fn update_status(&self) {
        let hunks = self.document.hunk_lines.len();
        let unresolved = self
            .comments()
            .iter()
            .filter(|comment| !comment.resolved)
            .count();
        let resolved = self.comments().len().saturating_sub(unresolved);
        if self.review_load_error.is_none() {
            self.review_count_label
                .set_text(&format!("{unresolved} OPEN · {resolved} RESOLVED"));
        }
        self.position_label
            .set_text(&self.current_line_position().unwrap_or_default());
        if let Some(error) = &self.review_save_error {
            self.status_label.add_css_class("zako-error");
            self.status_label
                .set_text(&format!("Review save failed: {error}"));
        } else if self.files.is_empty() {
            self.status_label.set_text("WORKING TREE CLEAN");
            self.hunk_label.set_text("");
        } else {
            self.status_label.set_markup(&format!(
                "<span foreground=\"{ADDED_FOREGROUND}\">+{}</span>    <span foreground=\"{REMOVED_FOREGROUND}\">−{}</span>",
                self.document.additions,
                self.document.deletions,
            ));
            if hunks == 0 {
                self.hunk_label.set_text("");
            } else {
                self.hunk_label.set_text(&format!(
                    "HUNK {:02} / {:02}",
                    self.current_hunk + 1,
                    hunks
                ));
            }
        }
    }

    fn current_line_position(&self) -> Option<String> {
        let line = self.document.lines.get(self.focused_diff_line?)?;
        match line.kind {
            crate::git::LineKind::Addition => line.new_line.map(|number| format!("NEW {number}")),
            crate::git::LineKind::Deletion => line.old_line.map(|number| format!("OLD {number}")),
            crate::git::LineKind::Context => match (line.old_line, line.new_line) {
                (Some(old), Some(new)) => Some(format!("OLD {old} / NEW {new}")),
                (Some(old), None) => Some(format!("OLD {old}")),
                (None, Some(new)) => Some(format!("NEW {new}")),
                (None, None) => None,
            },
            crate::git::LineKind::Hunk => Some("HUNK HEADER".to_owned()),
            crate::git::LineKind::Metadata => Some("DIFF HEADER".to_owned()),
        }
    }

    fn show_shortcuts(&self) {
        let help = gtk::Window::builder()
            .title("Zako keyboard shortcuts")
            .transient_for(&self.window)
            .modal(true)
            .default_width(380)
            .default_height(310)
            .build();
        let label = gtk::Label::new(Some(
            "← / →       move the text cursor by one character\n\
             ↑ / ↓       move the text cursor by one line\n\
             j / k       move one diff row; extend selection\n\
             ]f / [f    next / previous file\n\
             ]h / [h    next / previous hunk\n\
             ]c / [c    next / previous unresolved comment\n\
             gg          go to top\n\
             G           go to bottom\n\
             v           start line range selection\n\
             c           create or edit comment on focused line\n\
             Enter       save comment; edit focused comment\n\
             Backspace   delete character; delete an empty edited comment\n\
             C           toggle review panel\n\
             e           open current location in $EDITOR\n\
             r           refresh repository\n\
             Tab         toggle file sidebar\n\
             Ctrl+Enter  save comment\n\
             Esc         cancel selection or editor\n\
             ?           show this help\n\
             q           quit",
        ));
        label.set_xalign(0.0);
        label.set_yalign(0.0);
        label.set_margin_top(20);
        label.set_margin_bottom(20);
        label.set_margin_start(24);
        label.set_margin_end(24);
        help.set_child(Some(&label));
        let controller = gtk::EventControllerKey::new();
        controller.connect_key_pressed(|controller, key, _, _| {
            if key == gtk::gdk::Key::Escape {
                if let Some(widget) = controller.widget()
                    && let Some(window) = widget.root().and_downcast::<gtk::Window>()
                {
                    window.close();
                }
                gtk::glib::Propagation::Stop
            } else {
                gtk::glib::Propagation::Proceed
            }
        });
        help.add_controller(controller);
        help.present();
    }
}

#[derive(Clone)]
struct CommentTarget {
    file: String,
    start_line: usize,
    end_line: usize,
    side: DiffSide,
    anchor_hash: String,
    anchor_line_count: usize,
}

struct CommentDraft {
    id: Option<String>,
    target: Option<CommentTarget>,
}

fn launch_editor_in_terminal(
    editor: &str,
    editor_args: &[std::ffi::OsString],
) -> std::io::Result<std::process::Child> {
    let mut terminal = std::env::var("TERMINAL")
        .ok()
        .and_then(|value| shlex::split(&value))
        .filter(|parts| !parts.is_empty())
        .unwrap_or_default();
    if terminal.is_empty() {
        let candidates = [
            "foot",
            "ghostty",
            "kitty",
            "alacritty",
            "wezterm",
            "xterm",
            "kgx",
            "gnome-terminal",
        ];
        let Some(found) = candidates.iter().find_map(|name| find_executable(name)) else {
            return Err(std::io::Error::other(
                "no terminal emulator found; set $TERMINAL to launch a terminal editor",
            ));
        };
        terminal.push(found.to_string_lossy().into_owned());
    }

    let program = terminal.remove(0);
    let name = std::path::Path::new(&program)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("")
        .to_owned();
    let mut command = std::process::Command::new(program);
    command.args(terminal);
    if !command
        .get_args()
        .last()
        .is_some_and(|arg| matches!(arg.to_str(), Some("-e" | "--")))
    {
        match name.as_str() {
            "kitty" | "kgx" | "gnome-terminal" => {
                command.arg("--");
            }
            "wezterm" => {
                command.args(["start", "--"]);
            }
            _ => {
                command.arg("-e");
            }
        }
    }
    command.arg(editor).args(editor_args).spawn()
}

fn find_executable(name: &str) -> Option<PathBuf> {
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .map(|directory| directory.join(name))
        .find(|candidate| candidate.is_file())
}

static COMMENT_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

fn new_comment_id() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let counter = COMMENT_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!("{nanos:x}-{:x}-{counter:x}", std::process::id())
}

fn timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u64::MAX as u128) as u64
}

fn first_changed_line(document: &DiffDocument) -> Option<usize> {
    document.lines.iter().position(is_changed_line)
}

fn last_changed_line(document: &DiffDocument) -> Option<usize> {
    document.lines.iter().rposition(is_changed_line)
}

fn is_changed_line(line: &crate::git::DiffLine) -> bool {
    matches!(
        line.kind,
        crate::git::LineKind::Addition | crate::git::LineKind::Deletion
    )
}

#[derive(Clone)]
struct DiffLineAnchor {
    side: DiffSide,
    line: usize,
    text: String,
}

fn diff_line_anchor(document: &DiffDocument, index: usize) -> Option<DiffLineAnchor> {
    let line = document.lines.get(index)?;
    let (side, line_number) = match line.kind {
        crate::git::LineKind::Addition => (DiffSide::New, line.new_line?),
        crate::git::LineKind::Deletion => (DiffSide::Old, line.old_line?),
        _ => return None,
    };
    Some(DiffLineAnchor {
        side,
        line: line_number,
        text: line.text.clone(),
    })
}

fn find_anchored_line(document: &DiffDocument, anchor: &DiffLineAnchor) -> Option<usize> {
    let matches_side = |line: &crate::git::DiffLine| match anchor.side {
        DiffSide::New => line.kind == crate::git::LineKind::Addition,
        DiffSide::Old => line.kind == crate::git::LineKind::Deletion,
    };
    document
        .lines
        .iter()
        .enumerate()
        .find(|(_, line)| {
            matches_side(line)
                && line.new_line.or(line.old_line) == Some(anchor.line)
                && line.text == anchor.text
        })
        .or_else(|| {
            document
                .lines
                .iter()
                .enumerate()
                .filter(|(_, line)| matches_side(line) && line.text == anchor.text)
                .collect::<Vec<_>>()
                .into_iter()
                .reduce(|closest, candidate| {
                    let distance = |line: &crate::git::DiffLine| {
                        line.new_line
                            .or(line.old_line)
                            .unwrap_or_default()
                            .abs_diff(anchor.line)
                    };
                    if distance(candidate.1) < distance(closest.1) {
                        candidate
                    } else {
                        closest
                    }
                })
        })
        .or_else(|| {
            document
                .lines
                .iter()
                .enumerate()
                .filter(|(_, line)| matches_side(line))
                .min_by_key(|(_, line)| {
                    line.new_line
                        .or(line.old_line)
                        .unwrap_or_default()
                        .abs_diff(anchor.line)
                })
        })
        .map(|(index, _)| index)
}

fn begin_edit_comment(shared: &Rc<RefCell<AppState>>, id: &str) {
    if let Ok(mut app) = shared.try_borrow_mut() {
        app.start_edit_comment_by_id(id, shared);
    }
}

fn install_refresh_poll(state: &Rc<RefCell<AppState>>) {
    let state = state.clone();
    gtk::glib::timeout_add_local(Duration::from_millis(100), move || {
        let Ok(mut app) = state.try_borrow_mut() else {
            return gtk::glib::ControlFlow::Continue;
        };
        let mut received_event = false;
        if let Some(watcher) = &app.watcher {
            while watcher.events.try_recv().is_ok() {
                received_event = true;
            }
        }
        if received_event {
            app.last_fs_event = Some(Instant::now());
        }
        if app
            .last_fs_event
            .is_some_and(|last_event| last_event.elapsed() >= Duration::from_millis(350))
        {
            app.last_fs_event = None;
            app.refresh(&state);
        }
        gtk::glib::ControlFlow::Continue
    });
}
