use std::path::Path;

use gtk::prelude::*;
use sourceview::prelude::*;

use crate::git::{DiffDocument, LineKind};
use crate::theme::{ADDED_BACKGROUND, HUNK_FOREGROUND, REMOVED_BACKGROUND, SOURCE_STYLE_SCHEME};

pub struct DiffView {
    pub container: gtk::Box,
    pub heading: gtk::Label,
    pub source_view: sourceview::View,
    buffer: sourceview::Buffer,
    added_tag: gtk::TextTag,
    removed_tag: gtk::TextTag,
    hunk_tag: gtk::TextTag,
}

impl DiffView {
    pub fn new() -> Self {
        let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let heading = gtk::Label::new(Some("Select a changed file"));
        heading.set_xalign(0.0);
        heading.add_css_class("zako-file-heading");

        let buffer = sourceview::Buffer::new(None);
        let added_tag = gtk::TextTag::builder()
            .name("zako-added-line")
            .background(ADDED_BACKGROUND)
            .build();
        let removed_tag = gtk::TextTag::builder()
            .name("zako-removed-line")
            .background(REMOVED_BACKGROUND)
            .build();
        let hunk_tag = gtk::TextTag::builder()
            .name("zako-hunk-line")
            .foreground(HUNK_FOREGROUND)
            .build();
        let tag_table = buffer.tag_table();
        tag_table.add(&added_tag);
        tag_table.add(&removed_tag);
        tag_table.add(&hunk_tag);
        buffer.set_highlight_syntax(true);
        if let Some(scheme) = sourceview::StyleSchemeManager::default().scheme(SOURCE_STYLE_SCHEME)
        {
            buffer.set_style_scheme(Some(&scheme));
        }
        let source_view = sourceview::View::with_buffer(&buffer);
        source_view.set_editable(false);
        source_view.set_cursor_visible(false);
        source_view.set_monospace(true);
        source_view.set_show_line_numbers(false);
        source_view.set_tab_width(4);
        source_view.set_wrap_mode(gtk::WrapMode::None);
        source_view.add_css_class("zako-diff");
        source_view.set_left_margin(12);
        source_view.set_right_margin(12);
        source_view.set_top_margin(10);
        source_view.set_bottom_margin(10);

        let scrolled = gtk::ScrolledWindow::builder()
            .hexpand(true)
            .vexpand(true)
            .child(&source_view)
            .build();
        container.append(&heading);
        container.append(&scrolled);

        Self {
            container,
            heading,
            source_view,
            buffer,
            added_tag,
            removed_tag,
            hunk_tag,
        }
    }

    pub fn show_document(&self, path: Option<&Path>, document: &DiffDocument) {
        let Some(path) = path else {
            self.heading.set_text("No changed files");
            self.buffer.set_text("The working tree is clean.");
            return;
        };
        self.heading.set_text(&path.display().to_string());

        let manager = sourceview::LanguageManager::default();
        let language = manager
            .guess_language(Some(path), None)
            .or_else(|| manager.language("diff"));
        self.buffer.set_language(language.as_ref());
        self.buffer.set_highlight_syntax(true);

        if document.lines.is_empty() {
            self.buffer
                .set_text("No textual diff is available for this file.");
            return;
        }

        let rendered = document
            .lines
            .iter()
            .map(|line| {
                let old = line.old_line.map_or_else(String::new, |n| n.to_string());
                let new = line.new_line.map_or_else(String::new, |n| n.to_string());
                format!("{old:>5} {new:>5} │ {}", line.text)
            })
            .collect::<Vec<_>>()
            .join("\n");
        self.buffer.set_text(&rendered);

        for (index, line) in document.lines.iter().enumerate() {
            let Ok(line_number) = i32::try_from(index) else {
                continue;
            };
            let Some(start) = self.buffer.iter_at_line(line_number) else {
                continue;
            };
            let mut end = start;
            end.forward_to_line_end();
            let tag = match line.kind {
                LineKind::Addition => Some(&self.added_tag),
                LineKind::Deletion => Some(&self.removed_tag),
                LineKind::Hunk => Some(&self.hunk_tag),
                LineKind::Context | LineKind::Metadata => None,
            };
            if let Some(tag) = tag {
                self.buffer.apply_tag(tag, &start, &end);
            }
        }
    }

    pub fn scroll_to_line(&self, line: usize) {
        let Ok(line) = i32::try_from(line) else {
            return;
        };
        if let Some(mut iter) = self.buffer.iter_at_line(line) {
            self.source_view
                .scroll_to_iter(&mut iter, 0.0, false, 0.0, 0.0);
        }
    }

    pub fn scroll_to_top(&self) {
        let mut iter = self.buffer.start_iter();
        self.source_view
            .scroll_to_iter(&mut iter, 0.0, false, 0.0, 0.0);
    }

    pub fn scroll_to_bottom(&self) {
        let mut iter = self.buffer.end_iter();
        self.source_view
            .scroll_to_iter(&mut iter, 0.0, false, 0.0, 0.0);
    }

    pub fn scroll_by(&self, lines: f64) {
        let Some(scrolled) = self.container.last_child() else {
            return;
        };
        let Ok(scrolled) = scrolled.downcast::<gtk::ScrolledWindow>() else {
            return;
        };
        let adjustment = scrolled.vadjustment();
        let step = if adjustment.step_increment() > 0.0 {
            adjustment.step_increment()
        } else {
            20.0
        };
        let maximum = (adjustment.upper() - adjustment.page_size()).max(adjustment.lower());
        adjustment
            .set_value((adjustment.value() + step * lines).clamp(adjustment.lower(), maximum));
    }

    pub fn show_error(&self, message: &str) {
        self.heading.set_text("Unable to load diff");
        self.buffer.set_text(message);
    }
}
