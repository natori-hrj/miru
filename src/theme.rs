use gtk::gdk;

pub const ADDED_BACKGROUND: &str = "#18201b";
pub const REMOVED_BACKGROUND: &str = "#24191b";
pub const ADDED_FOREGROUND: &str = "#9db3a2";
pub const REMOVED_FOREGROUND: &str = "#bd999d";
pub const HUNK_FOREGROUND: &str = "#99979b";
pub const COMMENT_FOREGROUND: &str = "#c5c3c7";
pub const RESOLVED_COMMENT_FOREGROUND: &str = "#5f5e61";
pub const COMMENT_FOCUS_BACKGROUND: &str = "#242426";
pub const SOURCE_STYLE_SCHEME: &str = "oblivion";

const STYLESHEET: &str = r#"
window, .miru-root {
  background: #0b0b0c;
  color: #e0e0e1;
}
.miru-header {
  background: #101011;
  border-bottom: 1px solid #262628;
  min-height: 40px;
  padding: 0 14px;
}
.miru-sidebar-toggle {
  background: transparent;
  border: none;
  box-shadow: none;
  color: #777579;
  min-width: 24px;
  min-height: 24px;
  padding: 0 4px;
}
.miru-sidebar-toggle:hover {
  background: #19191a;
  color: #e0e0e1;
}
.miru-brand {
  color: #e0e0e1;
  font-size: 11pt;
  font-weight: 700;
}
.miru-header-separator {
  color: #4a494d;
  padding: 0 2px;
}
.miru-repo-name {
  color: #929094;
  font-family: monospace;
  font-size: 9pt;
}
.miru-file-count {
  color: #8f8d91;
  font-family: monospace;
  font-size: 9pt;
}
.miru-review-toggle, .miru-quiet-button {
  background: transparent;
  border: 1px solid transparent;
  box-shadow: none;
  color: #929094;
  min-height: 24px;
  padding: 2px 7px;
  font-size: 8pt;
}
.miru-review-toggle:hover, .miru-quiet-button:hover {
  background: #1b1b1d;
  border-color: #2d2d2f;
  color: #e0e0e1;
}
.miru-review-panel {
  background: #111112;
  border-left: 1px solid #2d2d2f;
}
.miru-review-header {
  background: #141415;
  border-bottom: 1px solid #2d2d2f;
  padding: 0 9px 0 1px;
}
.miru-review-file {
  color: #929094;
  font-family: monospace;
  font-size: 8.5pt;
  padding: 4px 2px;
}
.miru-comment-row {
  background: #171718;
  border-left: 2px solid #777579;
  padding: 8px;
}
.miru-comment-unresolved { border-left-color: #c5c3c7; }
.miru-comment-resolved { border-left-color: #4a494d; }
.miru-comment-stale { border-left-color: #898184; }
.miru-comment-location {
  color: #c5c3c7;
  font-family: monospace;
  font-size: 8pt;
}
.miru-comment-resolved .miru-comment-location { color: #5f5e61; }
.miru-comment-body {
  color: #d0ced2;
  font-size: 9pt;
}
.miru-comment-resolved .miru-comment-body { color: #777579; }
.miru-comment-stale .miru-comment-location { color: #aaa0a2; }
.miru-review-empty {
  color: #777579;
  font-size: 9pt;
  padding: 8px 2px;
}
.miru-review-status {
  color: #858388;
  font-family: monospace;
  font-size: 8pt;
  margin-right: 18px;
}
.miru-primary-action {
  background: #252527;
  border: 1px solid #49484b;
  color: #e0e0e1;
}
.miru-primary-action:hover { background: #303033; }
.miru-panel {
  background: #101011;
  border-right: 1px solid #262628;
}
.miru-file-list {
  background: #101011;
  padding: 2px 5px 8px;
}
.miru-file-list row {
  min-height: 30px;
  padding: 0 7px;
  margin: 1px 0;
  border-radius: 4px;
  border-left: 2px solid transparent;
}
.miru-file-list row:selected {
  background: #1c1c1f;
  border-left-color: #aaa8ac;
  outline: none;
}
.miru-file-list row:hover:not(:selected) {
  background: #171719;
}
.miru-section-title {
  color: #777579;
  font-size: 8pt;
  font-weight: 700;
  padding: 15px 14px 9px;
}
.miru-file-heading {
  background: #0b0b0c;
  border-bottom: 1px solid #202022;
  color: #c9c8ca;
  padding: 11px 16px;
  font-family: monospace;
  font-size: 9.5pt;
}
.miru-diff {
  background: #0b0b0c;
  color: #e0e0e1;
  font-family: "JetBrains Mono", monospace;
  font-size: 10pt;
}
.miru-diff text selection {
  background-color: #303033;
  color: #e0e0e1;
}
.miru-paned > separator {
  background: #262628;
  min-width: 1px;
}
.miru-status-line {
  background: #101011;
  border-top: 1px solid #262628;
  min-height: 31px;
  padding: 0 14px;
}
.miru-status {
  color: #a09ea2;
  font-family: monospace;
  font-size: 9pt;
}
.miru-hunk-status {
  color: #777579;
  font-family: monospace;
  font-size: 8pt;
}
.miru-added { color: #9db3a2; }
.miru-removed { color: #bd999d; }
.miru-error { color: #d6989b; }
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
