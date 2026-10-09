use gtk::gdk;

pub const ADDED_BACKGROUND: &str = "#18201b";
pub const REMOVED_BACKGROUND: &str = "#24191b";
pub const HUNK_FOREGROUND: &str = "#99979b";
pub const SOURCE_STYLE_SCHEME: &str = "oblivion";

const STYLESHEET: &str = r#"
window, .zako-root {
  background: #0b0b0c;
  color: #e0e0e1;
}
.zako-header {
  background: #141415;
  border-bottom: 1px solid #2d2d2f;
  min-height: 38px;
  padding: 0 12px;
}
.zako-title {
  color: #e0e0e1;
  font-weight: 600;
}
.zako-muted {
  color: #777579;
}
.zako-panel {
  background: #141415;
}
.zako-file-list {
  background: #141415;
}
.zako-file-list row {
  min-height: 28px;
  padding: 0 8px;
  border-radius: 3px;
}
.zako-file-list row:selected {
  background: #252527;
  outline: 1px solid #99979b;
  outline-offset: -1px;
}
.zako-file-list row:hover:not(:selected) {
  background: #1b1b1d;
}
.zako-section-title {
  color: #99979b;
  font-size: 0.9em;
  font-weight: 600;
  padding: 11px 12px 8px;
}
.zako-separator {
  background: #2d2d2f;
}
.zako-file-heading {
  background: #0b0b0c;
  border-bottom: 1px solid #2d2d2f;
  color: #c9c8ca;
  padding: 9px 13px;
  font-family: monospace;
}
.zako-diff {
  background: #0b0b0c;
  color: #e0e0e1;
  font-family: monospace;
  font-size: 10.5pt;
}
.zako-status {
  background: #141415;
  border-top: 1px solid #2d2d2f;
  color: #777579;
  min-height: 27px;
  padding: 0 10px;
  font-size: 0.9em;
}
.zako-added { color: #9db9a3; }
.zako-removed { color: #c39b9e; }
.zako-error { color: #df9b9d; }
"#;

pub fn install() {
    let Some(display) = gdk::Display::default() else {
        return;
    };
    let provider = gtk::CssProvider::new();
    provider.load_from_data(STYLESHEET);
    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
