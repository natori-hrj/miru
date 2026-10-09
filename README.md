# Zako

Zako is a lightweight desktop diff viewer for reviewing agent-written code.
It opens a local Git repository in a standalone GTK application and provides a
small review loop for leaving precise feedback for a coding agent.

## Motivation

AI coding agents can change several related files quickly. Zako keeps review
focused on those local changes: read the unified diff, attach comments to
changed lines, and export unresolved feedback for the agent. It is a native
Linux application, not a browser interface or a full Git client.

## Screenshot

<!-- Screenshot placeholder: add a capture of the GTK application here. -->

_Screenshot coming soon._

## Current features

- Modified, added, deleted, renamed, and untracked file list from Git's
  machine-readable status output.
- Unified diff view with old and new line numbers, hunk headers, context, and
  restrained addition and deletion backgrounds.
- GtkSourceView syntax highlighting selected from the changed file's type,
  with diff highlighting as a fallback.
- Debounced refresh when repository files change.
- Review comments on added and removed lines, including line ranges.
- Persistent comments with edit, delete, resolve, reopen, and stale-location
  handling.
- A hideable review panel with jumps to comment locations and Markdown copy.
- Markdown export and lightweight review list/status CLI commands.
- Open the selected changed line in `$EDITOR` with `e`.
- Read-only Git operations. Zako does not stage, commit, reset, check out, or
  discard changes. Review data is stored outside the working tree.

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

After installing the executable on your `PATH`, use `zako .` or `zako
/path/to/repository`. With no path, Zako uses the current directory. The
directory must belong to a Git repository.

### Agent review workflow

1. Let your coding agent modify the repository.
2. Open the repository with `zako .`.
3. Select a changed line, or press `v` and use `j` / `k` to select a range.
4. Press `c`, write a comment, and save with `Ctrl+Enter`.
5. Export unresolved comments as Markdown:

   ```bash
   zako review export
   ```

6. Send the exported review instructions back to your coding agent. The panel's
   **Copy unresolved** action copies the same Markdown to the Wayland clipboard.

Review comments are kept in `$XDG_DATA_HOME/zako/reviews/<repository-id>/`
(`~/.local/share/zako/reviews/<repository-id>/review.json` by default). The
repository ID is derived from the canonical repository path, so repositories
with the same directory name remain separate. Zako never writes review files
into the Git working tree.

The `e` shortcut launches the file and line through `$EDITOR`. Zako itself does
not edit source files; the chosen editor retains its normal editing behavior.
When `$EDITOR` is a terminal editor and Zako has no terminal attached, it uses
`$TERMINAL` or a detected terminal emulator.

### Review CLI

Run these from inside a repository, or pass a repository path:

```bash
zako review list [repository]
zako review status [repository]
zako review export [repository]
```

`export` writes unresolved comments to stdout and can be piped to another
tool. Stale comment locations are included and marked in the output.

## Keyboard shortcuts

| Shortcut | Action |
| --- | --- |
| `j` / `k`, `↓` / `↑` | Move through diff lines; extend a range while selecting |
| `]f` / `[f` | Next / previous changed file |
| `]h` / `[h` | Next / previous diff hunk |
| `]c` / `[c` | Next / previous unresolved comment |
| `gg` | Go to the top |
| `G` | Go to the bottom |
| `v` | Start a diff line range selection |
| `c` | Comment on the focused line or selected range |
| `Enter` | Edit the comment on the focused line, if any |
| `C` | Toggle the review panel |
| `e` | Open the focused file and line in `$EDITOR` |
| `r` | Refresh repository state |
| `Tab` | Toggle the changed-files sidebar |
| `Ctrl+Enter` | Save a review comment |
| `Esc` | Cancel selection or close the comment editor |
| `?` | Show shortcut help |
| `q` | Quit |

Arrow keys and mouse scrolling continue to work. Click a changed line to focus
it; drag across changed lines to select a range before adding a comment.

## Project status

Zako 0.2.0 is an early local review tool. It supports unified diffs and
agent-agnostic Markdown export. It does not connect to code hosting services,
modify Git state, or call AI services.

## Roadmap

- Optional side-by-side diff view.
- Optional reviewed-file tracking.
- More export formats for agent workflows.

## License

Zako is dual licensed under [MIT](LICENSE-MIT) or
[Apache License 2.0](LICENSE-APACHE), at your option.
