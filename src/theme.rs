use gtk::gdk;

pub const ADDED_BACKGROUND: &str = "#18201b";
pub const REMOVED_BACKGROUND: &str = "#24191b";
pub const ADDED_FOREGROUND: &str = "#9db3a2";
pub const REMOVED_FOREGROUND: &str = "#bd999d";
pub const HUNK_FOREGROUND: &str = "#99979b";
pub const SOURCE_STYLE_SCHEME: &str = "oblivion";

const STYLESHEET: &str = r#"
window, .zako-root {
  background: #0b0b0c;
  color: #e0e0e1;
}
.zako-header {
  background: #101011;
  border-bottom: 1px solid #262628;
  min-height: 40px;
  padding: 0 14px;
}
.zako-brand {
  color: #e0e0e1;
  font-size: 11pt;
  font-weight: 700;
}
.zako-header-separator {
  color: #4a494d;
  padding: 0 2px;
}
.zako-repo-name {
  color: #929094;
  font-family: monospace;
  font-size: 9pt;
}
.zako-file-count {
  color: #8f8d91;
  font-family: monospace;
  font-size: 9pt;
}
.zako-panel {
  background: #101011;
  border-right: 1px solid #262628;
}
.zako-file-list {
  background: #101011;
  padding: 2px 5px 8px;
}
.zako-file-list row {
  min-height: 30px;
  padding: 0 7px;
  margin: 1px 0;
  border-radius: 4px;
  border-left: 2px solid transparent;
}
.zako-file-list row:selected {
  background: #1c1c1f;
  border-left-color: #aaa8ac;
  outline: none;
}
.zako-file-list row:hover:not(:selected) {
  background: #171719;
}
.zako-section-title {
  color: #777579;
  font-size: 8pt;
  font-weight: 700;
  padding: 15px 14px 9px;
}
.zako-file-heading {
  background: #0b0b0c;
  border-bottom: 1px solid #202022;
  color: #c9c8ca;
  padding: 11px 16px;
  font-family: monospace;
  font-size: 9.5pt;
}
.zako-diff {
  background: #0b0b0c;
  color: #e0e0e1;
  font-family: "JetBrains Mono", monospace;
  font-size: 10pt;
}
.zako-paned > separator {
  background: #262628;
  min-width: 1px;
}
.zako-status-line {
  background: #101011;
  border-top: 1px solid #262628;
  min-height: 31px;
  padding: 0 14px;
}
.zako-status {
  color: #a09ea2;
  font-family: monospace;
  font-size: 9pt;
}
.zako-hunk-status {
  color: #777579;
  font-family: monospace;
  font-size: 8pt;
}
.zako-added { color: #9db3a2; }
.zako-removed { color: #bd999d; }
.zako-error { color: #d6989b; }
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
