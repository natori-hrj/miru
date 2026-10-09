# Zako

Zako is a lightweight desktop diff viewer for reviewing agent-written code.
It opens a local Git repository in a standalone GTK application and keeps the
current view up to date as files change.

## Motivation

AI coding agents often leave a working tree with several related changes.
Zako provides a focused place to read those changes without opening a full Git
client or leaving the desktop for a browser or terminal interface.

## Screenshot

<!-- Screenshot placeholder: add a capture of the GTK application here. -->

_Screenshot coming soon._

## Current features

- Modified, added, deleted, renamed, and untracked file list from Git's
  machine-readable status output.
- Unified diff view with old and new line numbers, hunk headers, context, and
  subtle addition and deletion backgrounds.
- GtkSourceView syntax highlighting selected from the changed file's type,
  with diff highlighting as a fallback.
- Debounced refresh when repository files change.
- Keyboard navigation and ordinary mouse selection and scrolling.
- Read-only Git access. Zako does not stage, commit, reset, check out, or discard
  changes.

## Install prerequisites on Arch Linux

Install Rust and the native GTK development packages:

```bash
sudo pacman -S --needed rustup gtk4 gtksourceview5 pkgconf git
rustup default stable
```

## Build

From the Zako source checkout, run:

```bash
cargo build --release
```

The executable is `target/release/zako`.

## Usage

Open the current repository or pass a repository path:

```bash
cargo run -- .
cargo run -- /path/to/repository
```

With no path, Zako uses the current directory. The directory must belong to a
Git repository. Git is invoked as a child process; Zako does not modify the
repository.

## Keyboard shortcuts

| Shortcut | Action |
| --- | --- |
| `j` / `k` | Scroll down / up |
| `]f` / `[f` | Next / previous changed file |
| `]h` / `[h` | Next / previous diff hunk |
| `gg` | Go to the top |
| `G` | Go to the bottom |
| `r` | Refresh repository state |
| `Tab` | Toggle the changed-files sidebar |
| `?` | Show shortcut help |
| `q` | Quit |

Arrow keys and mouse scrolling are available in the diff view as well.

## Project status

Zako is an early v0.1 implementation. It currently focuses on local repository
changes in a unified diff view. Binary files do not have an image preview, and
review comments, editor launching, and side-by-side diffs are not implemented.

## Roadmap

- Inline review comments stored outside the working tree.
- Export review comments as Markdown or JSON for coding agents.
- Open the selected file in `$EDITOR` at a changed line.
- Optional side-by-side diff view.

## License

Zako is dual licensed under [MIT](LICENSE-MIT) or
[Apache License 2.0](LICENSE-APACHE), at your option.
