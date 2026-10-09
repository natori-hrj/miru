use std::cell::{Cell, RefCell};
use std::path::Path;

use gtk::prelude::*;
use sourceview::prelude::*;

use crate::git::{DiffDocument, DiffLine, LineKind};
use crate::review::{DiffSide, ReviewComment};
use crate::theme::{
    ADDED_BACKGROUND, COMMENT_FOCUS_BACKGROUND, COMMENT_FOREGROUND, HUNK_FOREGROUND,
    REMOVED_BACKGROUND, RESOLVED_COMMENT_FOREGROUND, SOURCE_STYLE_SCHEME,
};

pub struct DiffView {
    pub container: gtk::Box,
    pub heading: gtk::Label,
    pub source_view: sourceview::View,
    buffer: sourceview::Buffer,
    added_tag: gtk::TextTag,
    removed_tag: gtk::TextTag,
    hunk_tag: gtk::TextTag,
    comment_tag: gtk::TextTag,
    resolved_comment_tag: gtk::TextTag,
    comment_body_tag: gtk::TextTag,
    focus_tag: gtk::TextTag,
    focus_marker_tag: gtk::TextTag,
    focused_document_line: Cell<Option<usize>>,
    document_to_buffer_line: RefCell<Vec<usize>>,
    buffer_to_document_line: RefCell<Vec<usize>>,
    focus_marker_offsets: RefCell<Vec<usize>>,
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
        let comment_tag = gtk::TextTag::builder()
            .name("zako-comment-line")
            .foreground(COMMENT_FOREGROUND)
            .build();
        let resolved_comment_tag = gtk::TextTag::builder()
            .name("zako-resolved-comment-line")
            .foreground(RESOLVED_COMMENT_FOREGROUND)
            .build();
        let comment_body_tag = gtk::TextTag::builder()
            .name("zako-comment-body-line")
            .foreground(COMMENT_FOREGROUND)
            .background("#171718")
            .build();
        let focus_tag = gtk::TextTag::builder()
            .name("zako-focused-diff-line")
            .background(COMMENT_FOCUS_BACKGROUND)
            .build();
        let focus_marker_tag = gtk::TextTag::builder()
            .name("zako-focused-diff-marker")
            .foreground("#e0e0e1")
            .build();
        let tag_table = buffer.tag_table();
        tag_table.add(&added_tag);
        tag_table.add(&removed_tag);
        tag_table.add(&hunk_tag);
        tag_table.add(&comment_tag);
        tag_table.add(&resolved_comment_tag);
        tag_table.add(&comment_body_tag);
        tag_table.add(&focus_tag);
        tag_table.add(&focus_marker_tag);
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
            comment_tag,
            resolved_comment_tag,
            comment_body_tag,
            focus_tag,
            focus_marker_tag,
            focused_document_line: Cell::new(None),
            document_to_buffer_line: RefCell::new(Vec::new()),
            buffer_to_document_line: RefCell::new(Vec::new()),
            focus_marker_offsets: RefCell::new(Vec::new()),
        }
    }

    pub fn show_document(
        &self,
        path: Option<&Path>,
        document: &DiffDocument,
        comments: &[ReviewComment],
    ) {
        self.focused_document_line.set(None);
        self.document_to_buffer_line.borrow_mut().clear();
        self.buffer_to_document_line.borrow_mut().clear();
        self.focus_marker_offsets.borrow_mut().clear();
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

        let mut rendered = Vec::new();
        let mut comment_body_lines = Vec::new();
        for (document_line, line) in document.lines.iter().enumerate() {
            self.document_to_buffer_line
                .borrow_mut()
                .push(rendered.len());
            let old = line.old_line.map_or_else(String::new, |n| n.to_string());
            let new = line.new_line.map_or_else(String::new, |n| n.to_string());
            self.focus_marker_offsets
                .borrow_mut()
                .push(old.len().max(5) + 1 + new.len().max(5) + 1);
            let focus_marker = if self.focused_document_line.get() == Some(document_line) {
                "▸"
            } else {
                " "
            };
            let marker = comment_marker(line, comments);
            rendered.push(format!(
                "{old:>5} {new:>5} {focus_marker}{marker} │ {}",
                line.text
            ));
            self.buffer_to_document_line
                .borrow_mut()
                .push(document_line);

            for comment in comments
                .iter()
                .filter(|comment| !comment.stale && comment_starts_on_line(comment, line))
            {
                let marker = if comment.resolved { "✓" } else { "●" };
                let body = comment
                    .body
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ");
                rendered.push(format!("            └─ {marker} {body}"));
                comment_body_lines.push(rendered.len() - 1);
                self.buffer_to_document_line
                    .borrow_mut()
                    .push(document_line);
            }
        }
        let rendered = rendered.join("\n");
        self.buffer.set_text(&rendered);

        for display_line in comment_body_lines {
            let Ok(line_number) = i32::try_from(display_line) else {
                continue;
            };
            if let Some(start) = self.buffer.iter_at_line(line_number) {
                let mut end = start;
                end.forward_to_line_end();
                self.buffer.apply_tag(&self.comment_body_tag, &start, &end);
            }
        }

        for (index, line) in document.lines.iter().enumerate() {
            let Some(buffer_line) = self.document_to_buffer_line.borrow().get(index).copied()
            else {
                continue;
            };
            let Ok(line_number) = i32::try_from(buffer_line) else {
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
            if let Some(comment) = matching_comment(comments, line) {
                let tag = if comment.resolved {
                    &self.resolved_comment_tag
                } else {
                    &self.comment_tag
                };
                self.buffer.apply_tag(tag, &start, &end);
            }
        }
    }

    pub fn select_display_range(&self, anchor: usize, focus: usize) {
        let document_to_buffer_line = self.document_to_buffer_line.borrow();
        if anchor >= document_to_buffer_line.len() || focus >= document_to_buffer_line.len() {
            return;
        }
        let low = document_to_buffer_line[anchor.min(focus)];
        let high = document_to_buffer_line[anchor.max(focus)];
        drop(document_to_buffer_line);
        let Some(start) = self.buffer.iter_at_line(low as i32) else {
            return;
        };
        let Some(mut end) = self.buffer.iter_at_line(high as i32) else {
            return;
        };
        end.forward_to_line_end();
        let buffer_start = self.buffer.start_iter();
        let buffer_end = self.buffer.end_iter();
        self.buffer
            .remove_tag(&self.focus_tag, &buffer_start, &buffer_end);
        for line in low..=high {
            if let Some(line_start) = self.buffer.iter_at_line(line as i32) {
                let mut line_end = line_start;
                line_end.forward_to_line_end();
                self.buffer
                    .apply_tag(&self.focus_tag, &line_start, &line_end);
            }
        }
        if focus >= anchor {
            self.buffer.select_range(&end, &start);
        } else {
            self.buffer.select_range(&start, &end);
        }
        self.set_focused_document_line(focus);
        self.source_view.grab_focus();
        self.scroll_to_line(focus);
    }

    pub fn clear_selection(&self) {
        let start = self.buffer.start_iter();
        let end = self.buffer.end_iter();
        self.buffer.remove_tag(&self.focus_tag, &start, &end);
        self.buffer.place_cursor(&start);
        if let Some(line) = self.focused_document_line.take() {
            self.set_focus_marker(line, false);
        }
    }

    pub fn focus_line(&self, line: usize) {
        self.select_display_range(line, line);
    }

    pub fn selected_display_range(&self) -> Option<(usize, usize)> {
        let (first, last) = self.buffer.selection_bounds()?;
        let buffer_to_document_line = self.buffer_to_document_line.borrow();
        let first = *buffer_to_document_line.get(first.line() as usize)?;
        let last = *buffer_to_document_line.get(last.line() as usize)?;
        Some((first.min(last), first.max(last)))
    }

    pub fn scroll_to_line(&self, line: usize) {
        let Some(line) = self.document_to_buffer_line.borrow().get(line).copied() else {
            return;
        };
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

    pub fn show_error(&self, message: &str) {
        self.heading.set_text("Unable to load diff");
        self.buffer.set_text(message);
        self.document_to_buffer_line.borrow_mut().clear();
        self.buffer_to_document_line.borrow_mut().clear();
        self.focus_marker_offsets.borrow_mut().clear();
    }

    pub fn document_line_at_buffer_line(&self, line: usize) -> Option<usize> {
        self.buffer_to_document_line.borrow().get(line).copied()
    }

    fn set_focused_document_line(&self, line: usize) {
        if let Some(previous) = self.focused_document_line.replace(Some(line))
            && previous != line
        {
            self.set_focus_marker(previous, false);
        }
        self.set_focus_marker(line, true);
    }

    fn set_focus_marker(&self, document_line: usize, focused: bool) {
        let Some(buffer_line) = self
            .document_to_buffer_line
            .borrow()
            .get(document_line)
            .copied()
        else {
            return;
        };
        let Some(marker_offset) = self
            .focus_marker_offsets
            .borrow()
            .get(document_line)
            .copied()
        else {
            return;
        };
        let Ok(buffer_line) = i32::try_from(buffer_line) else {
            return;
        };
        let Some(mut marker_start) = self.buffer.iter_at_line(buffer_line) else {
            return;
        };
        if !marker_start.forward_chars(marker_offset as i32) {
            return;
        }
        let mut marker_end = marker_start;
        if !marker_end.forward_char() {
            return;
        }
        self.buffer
            .remove_tag(&self.focus_marker_tag, &marker_start, &marker_end);
        self.buffer.delete(&mut marker_start, &mut marker_end);
        self.buffer
            .insert(&mut marker_start, if focused { "▸" } else { " " });
        if focused {
            let Some(mut marker_start) = self.buffer.iter_at_line(buffer_line) else {
                return;
            };
            if !marker_start.forward_chars(marker_offset as i32) {
                return;
            }
            let mut marker_end = marker_start;
            if marker_end.forward_char() {
                self.buffer
                    .apply_tag(&self.focus_marker_tag, &marker_start, &marker_end);
            }
        }
    }
}

fn comment_marker(line: &DiffLine, comments: &[ReviewComment]) -> &'static str {
    let mut matching = comments
        .iter()
        .filter(|comment| !comment.stale && comment_matches(comment, line));
    if matching
        .clone()
        .any(|comment| !comment.resolved && !comment.stale)
    {
        "●"
    } else if matching.any(|_| true) {
        "✓"
    } else {
        " "
    }
}

fn matching_comment<'a>(
    comments: &'a [ReviewComment],
    line: &DiffLine,
) -> Option<&'a ReviewComment> {
    comments
        .iter()
        .find(|comment| comment_matches(comment, line) && !comment.resolved && !comment.stale)
        .or_else(|| {
            comments.iter().find(|comment| {
                comment_matches(comment, line) && !comment.resolved && !comment.stale
            })
        })
        .or_else(|| {
            comments
                .iter()
                .find(|comment| comment_matches(comment, line) && !comment.stale)
        })
}

fn comment_matches(comment: &ReviewComment, line: &DiffLine) -> bool {
    let (line_number, side) = match line.kind {
        LineKind::Addition => (line.new_line, DiffSide::New),
        LineKind::Deletion => (line.old_line, DiffSide::Old),
        _ => (None, comment.side),
    };
    side == comment.side
        && line_number
            .is_some_and(|number| number >= comment.start_line && number <= comment.end_line)
}

fn comment_starts_on_line(comment: &ReviewComment, line: &DiffLine) -> bool {
    match comment.side {
        DiffSide::Old => {
            line.kind == LineKind::Deletion && line.old_line == Some(comment.start_line)
        }
        DiffSide::New => {
            line.kind == LineKind::Addition && line.new_line == Some(comment.start_line)
        }
    }
}
