# ghtui

A fast Git and GitHub workspace for the terminal, written in Rust with Ratatui.

- **Inbox**: your pull requests across every repository, with review, comments, checks and merge.
- **Repository**: history graph, diffs, trees at any revision, blame, staging and commits.
- **Search**: exact, regex and ranked search over your checkouts, plus GitHub code, issue and PR search.
- **Actions**: workflow runs, jobs, logs, artifacts, reruns, approvals and dispatch.
- **Dashboard**: PR and CI metrics that link back to the records behind them.

Every control works with both the keyboard and the mouse. ghtui uses your installed
`git` for local repositories and the GitHub CLI (`gh`) for GitHub, so it never asks
for a token of its own.

## Install

Linux (x86_64 and arm64):

```sh
curl -fsSL https://github.com/thowd22/ghtui/releases/latest/download/install.sh | bash
```

This URL works once the first release is published on that repository.
Until then, [build from source](#build-from-source).

The installer downloads a static binary for your CPU and checks its SHA-256
against the release's `SHA256SUMS`. It then installs a single file,
`~/.local/bin/ghtui`. It doesn't use sudo or edit your shell profile. It
needs Bash 3.2 or newer, `curl`, `tar`, `gzip`, `od` and `sha256sum` (or
`shasum`), which come with Linux and macOS. It doesn't need Rust, Node or
`jq`. If `~/.local/bin` isn't on your `PATH`, add it:

```sh
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc   # or ~/.zshrc
```

Installer options (each also has an environment variable):

| Option | Environment | Default |
| --- | --- | --- |
| `--version vX.Y.Z` | `GHTUI_VERSION` | latest release |
| `--install-dir DIR` | `GHTUI_INSTALL_DIR` | `~/.local/bin` |
| `--uninstall` | | removes the binary only |

Pass options through `bash -s --`, for example to pin a version:

```sh
curl -fsSL https://github.com/thowd22/ghtui/releases/latest/download/install.sh | bash -s -- --version v0.1.2
```

ghtui never updates itself. To upgrade, run the installer again: it installs
the latest release in place, and does nothing if that version is already
installed. To roll back, run it again with the older `--version`. "Latest" is
looked up once, and every file then comes from that exact version.

The installer also knows the macOS targets (Intel, and Apple silicon even
from a Rosetta shell), but release binaries for macOS aren't published yet.
On macOS it stops and points you to building from source (see below). Other
systems and CPUs get a clear "unsupported" error.

#### What the installer checks, and what it can't

- Downloads use HTTPS only, including redirects, with certificate checks and
  time and size limits. Your `~/.curlrc` is ignored.
- The archive must match its SHA-256 in the release's `SHA256SUMS`. It must
  hold exactly one regular file named `ghtui`, with no other paths, links or
  directories. That file's header must say it was built for your OS and CPU.
  Only after those checks is it run once, to confirm it reports the requested
  version. All of this happens before anything is installed. If a check
  fails, the binary you already had stays as it was.
- The new binary is written next to the old one and renamed over it, so an
  interrupted or concurrent install never leaves a half-written `ghtui`.
- It refuses to replace or uninstall a symlink, a directory, or a file that
  doesn't look like ghtui.

`SHA256SUMS` comes from the same release as the archive. It catches corrupt or
truncated downloads, but it can't protect you if the release itself has been
tampered with. When the repository is public, each release asset also has a
GitHub build provenance attestation that you can check with `gh`:

```sh
gh attestation verify ghtui-v0.1.2-x86_64-unknown-linux-musl.tar.gz --repo thowd22/ghtui
```

`curl | bash` runs whatever the server sends, so you are trusting that
release and GitHub's HTTPS. The script is written so that a download cut off
partway runs nothing. To read the script before running it, download it
first. `SHA256SUMS` lists `install.sh` too:

```sh
curl -fsSLO https://github.com/thowd22/ghtui/releases/latest/download/install.sh
curl -fsSLO https://github.com/thowd22/ghtui/releases/latest/download/SHA256SUMS
sha256sum --ignore-missing -c SHA256SUMS   # macOS: shasum -a 256 --ignore-missing -c SHA256SUMS
less install.sh
bash install.sh
```

To install by hand without the script, download `SHA256SUMS` and the archive
for your CPU from the release page, then:

```sh
sha256sum --ignore-missing -c SHA256SUMS
tar -xzf ghtui-v0.1.2-x86_64-unknown-linux-musl.tar.gz
install -m 755 ghtui ~/.local/bin/ghtui
```

### Requirements

- `git`
- [`gh`](https://cli.github.com/), signed in to your account:

  ```sh
  gh auth login
  gh auth status   # confirm
  ```

ghtui sends every GitHub request through `gh`, so whichever account and host `gh`
is signed in to is the one ghtui uses. For GitHub Enterprise, sign `gh` in to
that host and start ghtui with `--host your.host`.

### Build from source

Requires Rust 1.88 or newer. All dependencies are vendored, so the build needs no
network access after cloning:

```sh
git clone https://github.com/thowd22/ghtui
cd ghtui
cargo install --path . --locked --offline
```

That installs `ghtui` into `~/.cargo/bin`.

## Getting started

Run it from inside a Git checkout:

```sh
cd ~/code/my-project
ghtui
```

ghtui opens on that repository. If the checkout's `origin` points at GitHub, the
Actions, Search and Dashboard views default to that repository. The Inbox
always shows your pull requests from every repository.

```
ghtui [--repo OWNER/NAME] [--host HOST] [--config-dir PATH] [--no-color]
ghtui --help | --version
```

| Flag | Effect |
| --- | --- |
| `--repo OWNER/NAME` | Use this GitHub repository instead of the checkout's origin |
| `--host HOST` | GitHub host (default `github.com`); `gh` must be signed in to it |
| `--config-dir PATH` | Keep config and cache somewhere else (also `GHTUI_CONFIG_DIR`) |
| `--no-color` | Plain output with text labels instead of colors |

## Using ghtui

### Everywhere

| Key | Action |
| --- | --- |
| `1`–`5` | Inbox, Repository, Search, Actions, Dashboard |
| `?` | Help for the current view |
| `:` | Command palette: every action for this view and its key |
| `r` | Refresh |
| `s` | Settings |
| `Tab` / `Shift-Tab` | Move focus between the content and its controls |
| `Enter` or click | Open or activate the selection |
| `↑ ↓` / `j k`, wheel | Move and scroll; `PgUp`/`PgDn`, `Home`/`End` |
| `b` | Back (`←` also goes back from a pull request) |
| `Esc` | Close an overlay |
| `q` | Quit (`Ctrl-C` always exits) |

Every button on screen shows its key in brackets, for example `[l Logs]`. You can
click it or press the key. If something is unavailable, its button stays visible
and says what it needs first. When in doubt, press `:`.

Nothing is written without your say-so. Comments, reviews, merges, commits,
reruns and other changes first show a preview of the exact target and effect.
They happen only when you confirm with `Ctrl-S` or the Confirm button. Drafts are
saved locally, so closing a form or quitting doesn't lose your text.

### 1 Inbox: pull requests

- `r` loads your PRs. Filter the queues with `a` All, `c` Created by me,
  `m` Assigned to me and `v` Needs my review.
- `Enter` opens a PR, which has these tabs: Summary, Conversation, Commits,
  Files, Checks, Timeline, Metadata and Review.
- **Files:** select a diff line and press `i` to comment on it. `Shift-↑/↓` or a
  mouse drag extends the selection to a range, and `P` opens the comment draft.
  `K` comments on the whole file, and `T` opens threads to reply, resolve or
  reopen.
- **Checks:** opens the failing job's runs and logs in place.
- `F8` shows notifications, `F9` your review history and `F10` your reviews.

### 2 Repository: local Git

- Commit graph, the working tree, staged changes and branch diffs for the
  current checkout.
- `C` compares any two saved versions; `H` shows a file's history and `Y` its
  blame.
- `+` and `-` stage or unstage the selected file, lines or hunk, and `K`
  commits.
- `m` opens local operations: branches, remotes, stash, worktrees, rebase,
  merge and cherry-pick. Each shows a preview and needs confirmation.
- Browsing never changes your working files.

### 3 Search

- Choose a mode with `e` Exact, `x` Regex, `n` Ranked or `h` GitHub. Type the
  query, then open a result with `Enter` and its file or PR with `o`.
- Local modes search your working files, including uncommitted changes. You can
  also search an exact saved version.
- GitHub mode searches code, issues and PRs, scoped to the checkout's
  repository unless you choose another scope.
- `c` Corpus chooses which local repositories to search. Use **Map existing**
  to add a checkout you already have; nothing is cloned unless you ask.

### 4 Actions

- `R` lists runs and `W` lists workflows. `Enter` opens a run on its first
  failed job.
- `l` opens the highlighted job's logs. `>` and `<` jump between failures in
  jobs, steps or log lines, and `/` searches the logs.
- `a` lists artifacts, `j` lists jobs and `G` shows approval gates. Rerun,
  cancel, dispatch and the other changes go through preview and confirmation.

### 5 Dashboard

PR and CI metrics for the selected repositories over 7 or 30 days. Opening a
card lists the PRs, runs and reviews behind its number. If a value is partial
or unknown, the dashboard says so instead of showing zero.

## Files and privacy

- Configuration and the local cache live in `~/.config/ghtui/` (or
  `$XDG_CONFIG_HOME/ghtui`) on Linux and in
  `~/Library/Application Support/ghtui/` on macOS: `config.toml`,
  `cache.sqlite3`, the search `index/` and `avatars/`. Delete that directory to
  reset ghtui.
- To hide repositories, list them in `config.toml` and restart ghtui:

  ```toml
  excluded_repositories = ["owner/noisy-repo", "owner/archived-repo"]
  ```

  Their PRs leave the Inbox queues and counts, and they drop out of Search,
  Checks, the Dashboard and saved repository sets. ghtui may still read them
  from GitHub. Opening a local checkout of one in Repository still works.
- Private cached data is tied to the signed-in `gh` account. If you switch
  accounts or lose access, ghtui hides it again.
- Avatars come from GitHub's public avatar URLs without credentials, and
  initials are shown when no avatar is available. To turn avatars off, set
  `avatars = false` in `config.toml`.

## Uninstall

```sh
curl -fsSL https://github.com/thowd22/ghtui/releases/latest/download/install.sh | bash -s -- --uninstall
```

This removes the binary only, and only if it looks like ghtui. It never
follows a symlink. Pass the same `--install-dir DIR` if you installed
somewhere else. Your settings, cache, unsent drafts and repositories are
kept. They live in one directory, which you can delete yourself if you want
them gone:

- Linux: `${XDG_CONFIG_HOME:-~/.config}/ghtui`, which holds `config.toml`, the
  `cache.sqlite3` database (cached data and drafts), the search `index` and
  `avatars`
- macOS: `~/Library/Application Support/ghtui`
- or the directory you passed with `--config-dir`

Your Git repositories are never touched.

## Development

```sh
cargo run -- --help
node scripts/test-all.mjs                 # full suite: unit, provider and real-terminal tests
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

The terminal tests drive the real binary in a pseudo-terminal, using both the
keyboard and the mouse. They run against temporary Git repositories and an
offline fake `gh`, so they never touch your accounts. Contributor and agent
rules are in [AGENTS.md](AGENTS.md). Planned work is tracked with
[Backlog.md](https://github.com/MrLesk/Backlog.md): run `backlog board`.
The product specifications are in [backlog/docs](backlog/docs).

### CI and releases

`.github/workflows/ci.yml` runs formatting, Clippy, ShellCheck and the whole
test suite, including the terminal tests, on every push and pull request. Pull
requests get a read-only token and no secrets, and a failed run keeps its test
log as an artifact.

Pushing a `vX.Y.Z` tag that matches `Cargo.toml` runs
`.github/workflows/release.yml`. It builds static x86_64 and aarch64 Linux
binaries, each on a runner of the same CPU. It then installs each archive with
`scripts/install.sh` and runs the keyboard and mouse terminal tests against the
installed binary. Next it uploads a draft release, checks the uploaded files
byte for byte against the tested ones, and only then publishes it. To assemble
the assets locally without uploading anything, use
`scripts/package-release.sh`.

Local checks of these workflows only test their structure and the scripts they
call. They are not evidence of a hosted run. Native macOS builds are not part
of the release yet.
