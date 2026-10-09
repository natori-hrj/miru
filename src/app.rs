use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gtk::prelude::*;

use crate::git::{DiffDocument, Repository};
use crate::theme::{ADDED_FOREGROUND, REMOVED_FOREGROUND};
use crate::ui::{DiffView, build_file_list, install_shortcuts, make_file_row};
use crate::watcher::RepositoryWatcher;

pub struct AppState {
    pub window: gtk::ApplicationWindow,
    application: gtk::Application,
    repository: Repository,
    sidebar: gtk::Box,
    file_list: gtk::ListBox,
    diff_view: DiffView,
    count_label: gtk::Label,
    status_label: gtk::Label,
    hunk_label: gtk::Label,
    files: Vec<crate::git::ChangedFile>,
    selected_path: Option<PathBuf>,
    document: DiffDocument,
    current_hunk: usize,
    pending_key: Option<char>,
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
    let pane = gtk::Paned::new(gtk::Orientation::Horizontal);
    pane.add_css_class("zako-paned");
    pane.set_start_child(Some(&sidebar));
    pane.set_end_child(Some(&diff_view.container));
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
    status_line.append(&status_label);
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

    let state = Rc::new(RefCell::new(AppState {
        window: window.clone(),
        application: application.clone(),
        repository,
        sidebar,
        file_list: file_list.clone(),
        diff_view,
        count_label,
        status_label,
        hunk_label,
        files: Vec::new(),
        selected_path: None,
        document: DiffDocument::default(),
        current_hunk: 0,
        pending_key: None,
        watcher,
        last_fs_event: None,
    }));

    let callback_state = state.clone();
    file_list.connect_row_selected(move |_, row| {
        let Some(row) = row else { return };
        if let Ok(mut state) = callback_state.try_borrow_mut() {
            state.select_index(row.index() as usize);
        }
    });

    install_shortcuts(&window, &state);
    install_refresh_poll(&state);
    state.borrow_mut().refresh();
    window.present();
}

impl AppState {
    pub fn handle_key(&mut self, key: char) -> bool {
        if let Some(previous) = self.pending_key.take() {
            match (previous, key) {
                ('g', 'g') => {
                    self.diff_view.scroll_to_top();
                    return true;
                }
                (']', 'f') => {
                    self.move_file(1);
                    return true;
                }
                ('[', 'f') => {
                    self.move_file(-1);
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
            'j' => self.diff_view.scroll_by(3.0),
            'k' => self.diff_view.scroll_by(-3.0),
            'G' => self.diff_view.scroll_to_bottom(),
            'r' => self.refresh(),
            '?' => self.show_shortcuts(),
            'q' => self.application.quit(),
            _ => return false,
        }
        true
    }

    pub fn toggle_sidebar(&mut self) {
        self.sidebar.set_visible(!self.sidebar.is_visible());
    }

    fn move_file(&mut self, delta: isize) {
        if self.files.is_empty() {
            return;
        }
        let current = self
            .selected_path
            .as_ref()
            .and_then(|path| self.files.iter().position(|file| &file.path == path))
            .unwrap_or(0) as isize;
        let next = (current + delta).clamp(0, self.files.len() as isize - 1) as usize;
        self.select_index(next);
        self.file_list
            .select_row(self.file_list.row_at_index(next as i32).as_ref());
    }

    fn move_hunk(&mut self, delta: isize) {
        if self.document.hunk_lines.is_empty() {
            return;
        }
        let last = self.document.hunk_lines.len() - 1;
        self.current_hunk = (self.current_hunk as isize + delta).clamp(0, last as isize) as usize;
        self.diff_view
            .scroll_to_line(self.document.hunk_lines[self.current_hunk]);
        self.update_status();
    }

    fn select_index(&mut self, index: usize) {
        let Some(file) = self.files.get(index) else {
            return;
        };
        self.selected_path = Some(file.path.clone());
        self.current_hunk = 0;
        self.load_selected();
    }

    fn refresh(&mut self) {
        match self.repository.status() {
            Ok(snapshot) => {
                let selected_before = self.selected_path.clone();
                let changed_files = self.files != snapshot.files;
                self.files = snapshot.files;
                self.selected_path = selected_before
                    .filter(|path| self.files.iter().any(|file| &file.path == path))
                    .or_else(|| self.files.first().map(|file| file.path.clone()));

                if changed_files {
                    self.rebuild_file_list();
                }
                self.sync_selected_row();
                self.status_label.remove_css_class("zako-error");
                self.load_selected();
                self.count_label
                    .set_text(&format!("{} FILES", self.files.len()));
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
            self.diff_view.show_document(None, &self.document);
            self.update_status();
            return;
        };
        let Some(file) = self.files.iter().find(|file| file.path == path) else {
            return;
        };
        match self.repository.diff(file) {
            Ok(document) => {
                self.status_label.remove_css_class("zako-error");
                if self.document != document {
                    self.document = document;
                    self.current_hunk = self
                        .current_hunk
                        .min(self.document.hunk_lines.len().saturating_sub(1));
                    self.diff_view.show_document(Some(&path), &self.document);
                } else if self.diff_view.heading.text().as_str() != path.display().to_string() {
                    self.diff_view.show_document(Some(&path), &self.document);
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

    fn update_status(&self) {
        let hunks = self.document.hunk_lines.len();
        if self.files.is_empty() {
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

    fn show_shortcuts(&self) {
        let help = gtk::Window::builder()
            .title("Zako keyboard shortcuts")
            .transient_for(&self.window)
            .modal(true)
            .default_width(380)
            .default_height(310)
            .build();
        let label = gtk::Label::new(Some(
            "j / k       scroll down / up\n\
             ]f / [f    next / previous file\n\
             ]h / [h    next / previous hunk\n\
             gg          go to top\n\
             G           go to bottom\n\
             r           refresh repository\n\
             Tab         toggle file sidebar\n\
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
            app.refresh();
        }
        gtk::glib::ControlFlow::Continue
    });
}
