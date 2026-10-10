use gtk::prelude::*;

use crate::git::ChangedFile;

pub fn build_file_list() -> (gtk::Box, gtk::ListBox) {
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 0);
    panel.add_css_class("miru-panel");
    panel.set_size_request(235, -1);

    let title = gtk::Label::new(Some("CHANGES"));
    title.set_xalign(0.0);
    title.add_css_class("miru-section-title");

    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::Single);
    list.add_css_class("miru-file-list");
    let scrolled = gtk::ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .child(&list)
        .build();
    panel.append(&title);
    panel.append(&scrolled);
    (panel, list)
}

pub fn make_file_row(file: &ChangedFile) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    let line = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    line.set_valign(gtk::Align::Center);
    let mark = gtk::Label::new(Some(&file.marker.to_string()));
    mark.set_width_chars(1);
    mark.set_xalign(0.5);
    mark.add_css_class(match file.marker {
        'A' | '?' => "miru-added",
        'D' => "miru-removed",
        _ => "miru-muted",
    });
    let path = gtk::Label::new(Some(&file.display_path));
    path.set_xalign(0.0);
    path.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
    path.set_tooltip_text(Some(&file.display_path));
    path.set_hexpand(true);
    line.append(&mark);
    line.append(&path);
    row.set_child(Some(&line));
    row
}
