use std::rc::Rc;

use gtk::prelude::*;

pub fn present(
    parent: &gtk::ApplicationWindow,
    existing_body: Option<&str>,
    on_save: impl Fn(String) + 'static,
) {
    let window = gtk::Window::builder()
        .title(if existing_body.is_some() {
            "Edit review comment"
        } else {
            "New review comment"
        })
        .transient_for(parent)
        .modal(true)
        .default_width(480)
        .default_height(250)
        .build();

    let root = gtk::Box::new(gtk::Orientation::Vertical, 12);
    root.set_margin_top(16);
    root.set_margin_bottom(14);
    root.set_margin_start(16);
    root.set_margin_end(16);
    let editor = gtk::TextView::new();
    editor.set_wrap_mode(gtk::WrapMode::WordChar);
    editor.add_css_class("zako-comment-editor");
    let buffer = editor.buffer();
    if let Some(body) = existing_body {
        buffer.set_text(body);
    }
    let scrolled = gtk::ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .child(&editor)
        .build();
    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    actions.set_halign(gtk::Align::End);
    let cancel = gtk::Button::with_label("Cancel");
    let save = gtk::Button::with_label("Save  Ctrl+Enter");
    save.add_css_class("zako-primary-action");
    actions.append(&cancel);
    actions.append(&save);
    root.append(&scrolled);
    root.append(&actions);
    window.set_child(Some(&root));

    let on_save: Rc<dyn Fn(String)> = Rc::new(on_save);
    let save_comment: Rc<dyn Fn()> = Rc::new({
        let buffer = buffer.clone();
        let window = window.clone();
        let on_save = on_save.clone();
        move || {
            let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
            let body = text.trim().to_owned();
            if !body.is_empty() {
                on_save(body);
                window.close();
            }
        }
    });

    let save_comment_click = save_comment.clone();
    save.connect_clicked(move |_| save_comment_click());
    let cancel_window = window.clone();
    cancel.connect_clicked(move |_| cancel_window.close());

    let controller = gtk::EventControllerKey::new();
    controller.set_propagation_phase(gtk::PropagationPhase::Capture);
    let key_window = window.clone();
    controller.connect_key_pressed(move |_, key, _, modifiers| {
        if key == gtk::gdk::Key::Escape {
            key_window.close();
            gtk::glib::Propagation::Stop
        } else if key == gtk::gdk::Key::Return
            && modifiers.contains(gtk::gdk::ModifierType::CONTROL_MASK)
        {
            save_comment();
            gtk::glib::Propagation::Stop
        } else {
            gtk::glib::Propagation::Proceed
        }
    });
    window.add_controller(controller);
    window.present();
    editor.grab_focus();
}
