use gtk::prelude::*;

use crate::review::{ReviewComment, line_range};

pub struct ReviewPanel {
    pub container: gtk::Box,
    pub comments: gtk::Box,
    pub copy_button: gtk::Button,
}

pub struct CommentRow {
    pub jump: gtk::Button,
    pub edit: gtk::Button,
    pub resolve: gtk::Button,
    pub delete: gtk::Button,
}

impl ReviewPanel {
    pub fn new() -> Self {
        let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
        container.add_css_class("zako-review-panel");
        container.set_size_request(290, -1);

        let header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        header.add_css_class("zako-review-header");
        let title = gtk::Label::new(Some("REVIEW"));
        title.set_xalign(0.0);
        title.set_hexpand(true);
        title.add_css_class("zako-section-title");
        let copy_button = gtk::Button::with_label("Copy unresolved");
        copy_button.add_css_class("zako-quiet-button");
        copy_button.set_tooltip_text(Some("Copy unresolved comments as Markdown"));
        header.append(&title);
        header.append(&copy_button);

        let comments = gtk::Box::new(gtk::Orientation::Vertical, 8);
        comments.set_margin_top(10);
        comments.set_margin_bottom(10);
        comments.set_margin_start(10);
        comments.set_margin_end(10);
        comments.set_vexpand(true);
        let scrolled = gtk::ScrolledWindow::builder()
            .hexpand(true)
            .vexpand(true)
            .child(&comments)
            .build();
        container.append(&header);
        container.append(&scrolled);

        Self {
            container,
            comments,
            copy_button,
        }
    }

    pub fn clear(&self) {
        while let Some(child) = self.comments.first_child() {
            self.comments.remove(&child);
        }
    }

    pub fn append_file_heading(&self, file: &str) {
        let heading = gtk::Label::new(Some(file));
        heading.set_xalign(0.0);
        heading.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
        heading.set_tooltip_text(Some(file));
        heading.add_css_class("zako-review-file");
        self.comments.append(&heading);
    }

    pub fn append_empty(&self, message: &str) {
        let label = gtk::Label::new(Some(message));
        label.set_xalign(0.0);
        label.set_yalign(0.0);
        label.set_wrap(true);
        label.add_css_class("zako-review-empty");
        self.comments.append(&label);
    }

    pub fn append_comment(&self, comment: &ReviewComment) -> CommentRow {
        let row = gtk::Box::new(gtk::Orientation::Vertical, 5);
        row.add_css_class("zako-comment-row");
        if comment.resolved {
            row.add_css_class("zako-comment-resolved");
        } else if comment.stale {
            row.add_css_class("zako-comment-stale");
        } else {
            row.add_css_class("zako-comment-unresolved");
        }

        let marker = if comment.resolved {
            "✓"
        } else if comment.stale {
            "!"
        } else {
            "●"
        };
        let side = if comment.side == crate::review::DiffSide::Old {
            " · old"
        } else {
            ""
        };
        let location = gtk::Label::new(Some(&format!(
            "{marker} {}{side}{}",
            line_range(comment.start_line, comment.end_line),
            if comment.stale { " · stale" } else { "" },
        )));
        location.set_xalign(0.0);
        location.add_css_class("zako-comment-location");

        let body = gtk::Label::new(Some(comment.body.trim()));
        body.set_xalign(0.0);
        body.set_yalign(0.0);
        body.set_wrap(true);
        body.set_selectable(true);
        body.add_css_class("zako-comment-body");

        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        let jump = gtk::Button::with_label("Jump");
        let edit = gtk::Button::with_label("Edit");
        let resolve = gtk::Button::with_label(if comment.resolved {
            "Reopen"
        } else {
            "Resolve"
        });
        let delete = gtk::Button::with_label("Delete");
        for button in [&jump, &edit, &resolve, &delete] {
            button.add_css_class("zako-quiet-button");
        }
        actions.append(&jump);
        actions.append(&edit);
        actions.append(&resolve);
        actions.append(&delete);

        row.append(&location);
        row.append(&body);
        row.append(&actions);
        self.comments.append(&row);
        CommentRow {
            jump,
            edit,
            resolve,
            delete,
        }
    }
}
