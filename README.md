# Ferrocommander

A keyboard-centric dual-pane file manager for Linux, Windows and macOS, in
the style of [Total Commander](https://www.ghisler.com/). Two directories side by side,
every operation on a key, and no waiting.

Built from scratch in Rust and GTK4.

## Install on Ubuntu

The latest build of `main` is published as a `.deb`:

```sh
wget -P /tmp https://github.com/pirx42/ferrocommander/releases/download/rolling/ferrocommander-linux-x86_64.deb
sudo apt install /tmp/ferrocommander-linux-x86_64.deb
```

`/tmp` rather than the directory you happen to be in, because Ubuntu creates
home directories as `0750`: `apt` drops its download sandbox for a file the
unprivileged `_apt` user cannot reach and says so in four lines of
`pkgAcquire::Run (13: Permission denied)`. The install works either way — the
note is not a failure — but the quiet path is the one worth writing down.

The filename carries no version on purpose: the release is rolling, and a URL
worth putting in a README is one that does not change every commit. The
version is in the package — `apt show ferrocommander` after installing, or
`dpkg -I` before.

`apt install ./file.deb` rather than `dpkg -i`, because it pulls in the GTK 4
runtime; `dpkg` will refuse and leave the package half-configured.

**Ubuntu 24.04 or later.** The pane needs a GTK API that arrived in 4.12, and
22.04 ships 4.6. Details, and the rest of the packaging:
[docs/packaging.md](docs/packaging.md).

The release is *rolling* — one tag, `rolling`, replaced on every commit,
always holding the newest build that passed the full test suite. There are no
versioned releases yet.

Not called `main`: a tag with the same name as a branch makes `git push
origin main` fail with "src refspec main matches more than one", because git
cannot tell which of the two is meant.

## Run on Windows

The same commit, as a zip that unpacks into one folder:

```
https://github.com/pirx42/ferrocommander/releases/download/rolling/ferrocommander-windows-x86_64.zip
```

Unpack it anywhere and run **`ferrocommander.cmd`** — not the `.exe` beside
it. The `.cmd` sets one environment variable the app needs in order to start
at all; started directly, the `.exe` exits after about six seconds without
ever presenting a window ([docs/ui-shell.md](docs/ui-shell.md) § *The renderer
on Windows*).

Nothing is installed. Everything GTK needs is in the folder, there is no
runtime to fetch first, and deleting the folder removes the program. The one
thing written outside it is the settings file, at
`%APPDATA%\ferrocommander\config.toml`.

Built and run on Windows 11. Two things do not work there yet — the command
line runs nothing, and the status line shows no free-space figure — and both
are written down with the rest of it in
[docs/windows.md](docs/windows.md).

## Run on macOS (Apple Silicon)

The same commit, as a zip holding an `.app`:

```
https://github.com/pirx42/ferrocommander/releases/download/rolling/ferrocommander-macos-arm64.zip
```

Unpack it, put **FerroCommander.app** wherever you like, and start it with a
right-click (Control-click) and *Open* the first time: the app is not
notarized with Apple, so a plain double-click on a downloaded copy is refused
by Gatekeeper with a message that does not mention this way around it. Once
opened that way, it opens normally ever after.

Everything the app needs is inside the bundle, and deleting it removes the
program. Honesty about the state of it: the bundle is built and
smoke-started on a real Mac on every commit, but no person has used the app
on macOS yet — what a first real session should look at is written down in
[docs/future-improvements.md](docs/future-improvements.md).

## What it does

| | |
|---|---|
| Two panes | `Tab` between them, `Ctrl+←`/`→` to clone one into the other, `Ctrl+U` to swap |
| The function keys | `F3` view, `F4` edit, `F5` copy, `F6` move, `F7` new directory, `F8` delete |
| Marking | `Space`, `Insert`, the `Num` keys, by wildcard, by extension — and `Num /` brings back what the last operation spent |
| Archives | `Enter` on a `.zip`, `.tar` or `.tar.gz` walks into it like a folder; `Alt+F5` packs |
| Search | `Alt+F7`, by name or content, results streaming as they are found |
| `Ctrl+Q` | quick view: the other pane shows whatever the cursor is on |
| `Ctrl+B` | the whole tree below a pane as one flat list |
| `Ctrl+D` | favourite directories, maintained from inside the list |
| `Alt+Shift+Enter` | count what the marked folders actually hold |
| `Ctrl+M` | rename many files by a rule, with the preview *being* the rename |

Every key: [docs/keymap.md](docs/keymap.md). They are configurable, in
[the settings file](docs/config.md).

## Two things it is serious about

**Speed.** Where a decision trades speed against a prettier surface or a
tidier abstraction, speed wins. Every performance claim in this repository
carries a measurement, and the things that are still slow are written down:
[docs/performance.md](docs/performance.md).

**Not losing your files.** A file manager is trusted with the only copy of
things, and the failure that matters is not a crash but a job that reports
success over a file it destroyed. The awkward corner gets a test rather than
the benefit of the doubt: [docs/reliability.md](docs/reliability.md).

## Build from source

```sh
sudo apt install build-essential pkg-config libgtk-4-dev xvfb xdotool
cargo run -p fc-app
```

`xvfb` and `xdotool` are for the end-to-end tests, which drive the real binary
with real key presses. They are not optional: without them
`cargo test --workspace` fails by design, because a UI test that quietly skips
is worse than no UI test.

Before committing anything, `scripts/green-gate.sh` — format, lint, the
Windows and macOS lint branches, every test, the release build, the
documentation links.

## Layout

| | |
|---|---|
| `crates/fc-core` | the engine: filesystem, listing model, file operations, archives, search. No GTK, headless-testable |
| `crates/fc-app` | the GTK4 shell. Never touches the filesystem directly |
| `docs/` | one file per topic, [indexed here](docs/CLAUDE.md) |

The split is the project's central boundary, and what makes archives
browsable folders for free: a pane just holds a different filesystem.
[crates/CLAUDE.md](crates/CLAUDE.md) has the argument.

## Licence

MIT.
