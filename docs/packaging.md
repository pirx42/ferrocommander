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

## What was checked by hand, once

Built here, installed with `dpkg -i`, started under Xvfb — the window came up
titled `FerroCommander #121 (fffae6b)`, which is the version agreement above,
observed — and removed with `dpkg -r`, which left none of the three files
behind.
