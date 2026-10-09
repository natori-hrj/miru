use gtk::prelude::*;

use crate::app::AppState;

pub fn install_shortcuts(
    window: &gtk::ApplicationWindow,
    state: &std::rc::Rc<std::cell::RefCell<AppState>>,
) {
    let controller = gtk::EventControllerKey::new();
    controller.set_propagation_phase(gtk::PropagationPhase::Capture);
    let state = state.clone();
    controller.connect_key_pressed(move |_, key, _, _modifiers| {
        let composing_comment = state.borrow().is_composing_comment();
        if composing_comment {
            if key == gtk::gdk::Key::Escape {
                state.borrow_mut().cancel_pending_comment(&state);
                return gtk::glib::Propagation::Stop;
            }
            if key == gtk::gdk::Key::Return || key == gtk::gdk::Key::KP_Enter {
                state.borrow_mut().save_pending_comment(&state);
                return gtk::glib::Propagation::Stop;
            }
            if key == gtk::gdk::Key::BackSpace && state.borrow_mut().backspace_empty_comment(&state)
            {
                return gtk::glib::Propagation::Stop;
            }
            return gtk::glib::Propagation::Proceed;
        }

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

        let Some(character) = key.to_unicode() else {
            return gtk::glib::Propagation::Proceed;
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
