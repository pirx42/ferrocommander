# Packaging — the Ubuntu `.deb`

← Parent: [CLAUDE.md](CLAUDE.md)

A package somebody can download and install, rebuilt on every commit to
`main`. The audience stops being "a developer with `cargo`" and becomes
"somebody with Ubuntu", which is what everything here follows from.

## The logic is in a script, not in the workflow

`scripts/package-deb.sh` builds the package and checks it.
`.github/workflows/main.yml` installs the build dependencies and calls it.

**That split is the point.** A GitHub workflow can only be tested by pushing
it — there is no local run, no probe, no green gate. So the workflow holds as
close to nothing as it can, and everything real lives in a script that runs on
a laptop. It is `scripts/green-gate.sh`'s argument one level out: a sequence
nobody can run is a sequence nobody can check.

## What the package holds

| | |
|---|---|
| `/usr/bin/ferrocommander` | the binary. The crate is `tc-app` — its half of the tc-core/tc-app split — but `/usr/bin/tc-app` is a name nobody would have typed on purpose, so a `[[bin]]` section renames what ships without touching the crate. |
| `/usr/share/applications/st.rose.Ferrocommander.desktop` | the launcher entry, named after the application id the shell already registers. |
| `/usr/share/icons/hicolor/scalable/apps/st.rose.Ferrocommander.svg` | the icon: two panes, one brighter, because one pane always has the keyboard. |

The description and the file list are `[package.metadata.deb]` in
`crates/tc-app/Cargo.toml`, beside the crate they describe.

## The dependencies are derived, not written

`depends = "$auto"` runs `dpkg-shlibdeps` over the built binary, the way
Debian's own tooling does. A hand-maintained list for a GTK program is a list
that goes stale the first time a dependency moves.

It produces, today:

```
libc6 (>= 2.39), libglib2.0-0t64 (>= 2.54.0), libgtk-4-1 (>= 4.12.0), libpango-1.0-0 (>= 1.14.0)
```

**`libgtk-4-1 (>= 4.12.0)` is the floor this project already documented**,
arrived at independently from the built ELF rather than from the manifest —
which is a pleasant confirmation that the `v4_12` feature and the runtime
requirement agree.

## The version is the one in the title bar

`0.1.0-<commit count>`: the crate version, and the same
`git rev-list --count HEAD` that `crates/tc-app/build.rs` stamps into the
window title. So `dpkg -l` and a screenshot agree about which build somebody
is running — `0.1.0-121` in the package list is `#121` in the title.

Zero when git cannot answer — a source tarball, an image with no git — which
sorts below every real build rather than failing.

**The rule lives here, because the code cannot share it.** `build.rs` runs
inside a Rust build script that has to work on Windows with no shell;
`package-deb.sh` runs before cargo is invoked at all. Neither can call the
other, so the one line — `git rev-list --count HEAD` — is written in both, and
this paragraph is the place that says they must agree. If the rule ever
changes, it changes in two files.

## Ubuntu 24.04, and why not 22.04

24.04 ships GTK 4.14; 22.04 ships 4.6, and this needs 4.12
([the manifest records that trade](../Cargo.toml)). A package built on 24.04
installs on 24.04 and later. The workflow pins `ubuntu-24.04` rather than
`ubuntu-latest` for exactly that reason: `latest` moves, and the day it does,
the package quietly stops installing on the oldest release it supports.

## What the script checks, and why those three

Not a test suite — a package has no unit tests — but the three things that
have been wrong in somebody's package before:

- **Something is declared.** An empty `Depends:` installs cleanly on a
  machine with no GTK and then fails to start.
- **Everything is in there.** The binary, the entry and the icon, by path.
- **The desktop entry is valid**, by `desktop-file-validate` where it exists.
  An invalid one does not stop the installation; it stops the launcher entry
  appearing, silently.

## The workflow, and what could be checked about it before it ran

`.github/workflows/main.yml`: on a push to `main` (or by hand), Ubuntu 24.04,
install the dependencies, run the gate, run the packaging script, replace the
release. `contents: write` and nothing else; `concurrency` cancels an
in-flight run when a second commit arrives, because the release only ever
holds the newest build.

The file cannot be run before it is pushed. Three things about it *could* be
checked here, and were:

- **It parses**, and every `run:` block is valid shell (`bash -n`).
- **The release step does what it reads like.** Run with `gh` stubbed out,
  the SHA reaches `--target`, the notes stay one argument, and the glob finds
  the built `.deb`.
- **The scripts it calls are executable in the index**, `100755`, which a
  workflow discovers by failing.

One line was removed by reading rather than by running: the gate was wrapped
in `xvfb-run`, and the end-to-end harness starts an `Xvfb` of its own per test
on its own display numbers — so the outer server would have been one nothing
connects to.

**What is left unchecked** is everything that needs GitHub: whether the runner
image has what the `apt-get` line assumes, whether `gh release delete` and
`create` in sequence are reliable, and whether `fetch-depth: 0` really gives
`build.rs` the commit count. Those are watched on the first real run.

## The asset has no version in its name

The release is rolling — one tag, replaced every commit — so its download URL
has to be one somebody can put in a README and not revisit. GitHub's URLs end
in the asset's own filename, and Debian convention puts the version there,
which would change the URL on every build.

So `package-deb.sh` hard-links the built package to
`ferrocommander_amd64.deb`, and that is what gets uploaded. It costs nothing,
and nothing is lost: the version was never in the filename to begin with, it
is in the package, where `dpkg -I` and `apt show` read it.

## What was checked by hand, once

Built here, installed with `dpkg -i`, started under Xvfb — the window came up
titled `FerroCommander #121 (fffae6b)`, which is the version agreement above,
observed — and removed with `dpkg -r`, which left none of the three files
behind.
