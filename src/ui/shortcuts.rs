use gtk::prelude::*;

use crate::app::AppState;

pub fn install_shortcuts(
    window: &gtk::ApplicationWindow,
    state: &std::rc::Rc<std::cell::RefCell<AppState>>,
) {
    let controller = gtk::EventControllerKey::new();
    controller.set_propagation_phase(gtk::PropagationPhase::Capture);
    let state = state.clone();
    controller.connect_key_pressed(move |_, key, _, _| {
        if key == gtk::gdk::Key::Tab {
            state.borrow_mut().toggle_sidebar();
            return gtk::glib::Propagation::Stop;
        }
        if key == gtk::gdk::Key::Escape {
            state.borrow_mut().handle_key('\u{1b}', &state);
            return gtk::glib::Propagation::Stop;
        }
        if key == gtk::gdk::Key::Return {
            state.borrow_mut().handle_key('\n', &state);
            return gtk::glib::Propagation::Stop;
        }
        if let Some(focus) = gtk::prelude::RootExt::focus(&state.borrow().window)
            && let Ok(text_view) = focus.downcast::<gtk::TextView>()
            && text_view.is_editable()
        {
            return gtk::glib::Propagation::Proceed;
        }

        let character = match key {
            gtk::gdk::Key::Down => 'j',
            gtk::gdk::Key::Up => 'k',
            _ => {
                let Some(character) = key.to_unicode() else {
                    return gtk::glib::Propagation::Proceed;
                };
                character
            }
        };

        let mut app = state.borrow_mut();
        if app.handle_key(character, &state) {
            gtk::glib::Propagation::Stop
        } else {
            gtk::glib::Propagation::Proceed
        }
    });
    window.add_controller(controller);
}
