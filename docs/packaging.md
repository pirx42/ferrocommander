# Packaging — the Ubuntu `.deb`, the Windows zip, and the macOS app

← Parent: [CLAUDE.md](CLAUDE.md)

Something somebody can download and run, rebuilt on every commit to `main`.
The audience stops being "a developer with `cargo`" and becomes "somebody with
Ubuntu", "somebody with Windows" or "somebody with a Mac", which is what
everything here follows from — including the three being shaped so
differently, because those people expect completely different things.

## The logic is in a script, not in the workflow

`scripts/package-deb.sh`, `scripts/package-windows.sh` and
`scripts/package-macos.sh` build a package each and check it.
`.github/workflows/main.yml` installs the build dependencies and calls them.

**That split is the point.** A GitHub workflow can only be tested by pushing
it — there is no local run, no probe, no green gate. So the workflow holds as
close to nothing as it can, and everything real lives in a script that runs on
a laptop. It is `scripts/green-gate.sh`'s argument one level out: a sequence
nobody can run is a sequence nobody can check.

The Windows half makes the argument harder to ignore. Nothing about a GTK
bundle for Windows can be checked from Linux at all — not the DLL list, not
the schemas, not whether the thing starts — so the laptop that runs the script
is not a convenience there, it is the only place the work exists before a
runner sees it.

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

**The rule is in `scripts/version.sh`, and in one other place it cannot
reach.** `build.rs` runs inside a Rust build script that has to work on
Windows with no shell, so it cannot source a shell script; both packaging
scripts can, and do. That leaves the one line — `git rev-list --count HEAD` —
written twice rather than three times, and this paragraph is what says the two
must agree. If the rule ever changes, it changes in two files.

A third copy was what adding the Windows package would have cost if the rule
had stayed where it was, which is what moved it: two places nobody can avoid
is a constraint, three where the third was optional is just duplication
(skill [44](skills/44-no-redundancy.md)).

The zip has no `dpkg -I` to read the version back out of, so it goes in the
name of the folder inside the archive — `ferrocommander-0.1.0-186\` — while
the archive itself keeps a fixed name, for the reason under *The asset has no
version in its name* below.

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

## The Windows zip: no installer, one folder

Windows gets a zip that unpacks into a single folder and runs from wherever it
lands. No installer, no registry keys, no Start-menu entry: deleting the
folder removes the program. That is not a corner cut — it is what this
program's audience expects, because Total Commander itself ships that way.

| | |
|---|---|
| `ferrocommander.exe` | the binary, built by the `x86_64-pc-windows-gnu` toolchain, which is the only one that can link MSYS2's GTK4 ([windows.md](windows.md)). |
| `ferrocommander.cmd` | what a person actually runs. Its own section, below. |
| `*.dll` | the GTK4 runtime, found rather than listed. The section after that. |
| `share/glib-2.0/schemas/gschemas.compiled` | GTK reads its own settings through GSettings and **aborts at startup** without this. Compiled by the script rather than copied out of the MSYS2 prefix, so there is one code path that is always right instead of a copy that depends on a package's post-install hook having run. |
| `README.txt` | six lines saying to run the `.cmd`. |

**No icon theme and no gdk-pixbuf loaders**, which is where a GTK bundle
usually spends most of its megabytes. Not an oversight, and not a saving taken
on a hunch: the app names no icon and loads no image — a grep for `icon_name`,
`IconTheme`, `Pixbuf` and `Image::` over `crates/tc-app/src` finds nothing —
and the iconography GTK's own widgets use is compiled into `libgtk-4-1.dll` as
a GResource. The smoke test below is what stops that quietly becoming false.

## The DLLs are derived, not written

The same argument as `depends = "$auto"` one platform over, and it bites
harder here. `package-windows.sh` reads the binary's imports with `objdump`,
copies every DLL that MSYS2 provides, and repeats over each one it copied
until nothing new turns up. What is left unfound is a Windows system DLL —
`kernel32`, `user32`, `msvcrt` — which every Windows already has and none of
which may be shipped.

A hand-written list would go stale the first time a dependency moved, and here
it would go stale **silently**: the build machine has the DLL on its `PATH`,
so a bundle missing it works perfectly for the person who built it and fails
for everybody who downloads it. That is the failure this is written against.

`objdump` rather than `ntldd`, because `objdump` comes with the toolchain
group [windows.md](windows.md) already installs. One fewer package to be
missing on a fresh machine is one fewer way for this to fail there and nowhere
else.

## Why there is a `.cmd` next to the `.exe`

The app does not start under the renderer GTK picks for itself: it exits about
six seconds in with `0xC0000005`, silently, without ever presenting a window
([ui-shell.md](ui-shell.md) § *The renderer on Windows*). The launcher sets
`GSK_RENDERER=cairo` and runs the binary beside it, found through `%~dp0` so
the working directory does not matter.

Set there rather than in `main()`, because the binary is not only a Windows
binary. Hard-coding a renderer in the code would impose one platform's
unexplained crash on the two platforms where GTK's own default is the right
answer — and it would bury the workaround where nobody reading a bug report
would find it. In the launcher it is six lines of `rem` above the line that
does it.

The value is named **once in the script**: written into the launcher, and
exported for the smoke test that starts the app. So the setting that ships and
the setting that was tested are the same string by construction rather than by
somebody remembering to change both.

What that leaves is the `.exe` sitting in the folder as a trap for whoever
double-clicks it, which is what the `README.txt` is for.

## What the Windows script checks, and why those three

The same three the `.deb` checks, in the forms Windows offers them:

- **Nothing declared.** Every DLL the staged files import is either in the
  bundle or is not one MSYS2 provides. Walked a second time, flat over the
  staged folder rather than reusing the queue that filled it: a check that
  reuses the traversal it is checking cannot catch that traversal being wrong.
- **Everything is in there.** The binary, the launcher, the notes and the
  compiled schemas, by path.
- **It starts, and stays started.** The counterpart of
  `desktop-file-validate` — what it catches does not announce itself, it just
  means nobody can run what was published. The app runs under a `timeout` and
  has to still be running when the clock runs out.

Twenty seconds for that last one, because the documented failure is not a
refusal to start but a death about six seconds in. A check that only asked
"did it launch" would pass over exactly the bug it is there for.

It is a `timeout` rather than a background job and a `kill -0`, which is how
it was written first and was wrong: a child that has exited but has not been
reaped still answers signal 0, so the check meant to catch the app dying would
have reported a corpse as alive. The verdict is a number now — 124, `timeout`
saying it had to stop the app itself — and there is nothing subtle left in it.

## The macOS app: a bundle that stands alone

macOS (Apple Silicon) gets a zip holding a minimal `.app` — what lands in
`/Applications` and what Finder treats as a program — built natively on
GitHub's arm64 runners, which are real Macs. That sentence carries more than
it looks like: the smoke test inside `package-macos.sh` was **the first time
ferrocommander ever ran on macOS at all** (CI run #24, 2026-08-31; the
[plan](plans/2026-08-31-macos-package.md) is the record).

| | |
|---|---|
| `Contents/MacOS/ferrocommander` | the launcher, and what `Info.plist` names as the executable. Its own section, below. |
| `Contents/MacOS/ferrocommander-bin` | the binary, native `aarch64-apple-darwin`. Not stripped, for the Windows zip's reason: a first platform's crash reports are worth something only with symbols in them. |
| `Contents/Frameworks/*.dylib` | the GTK4 runtime — forty dylibs today — found and *rewritten*, the section below. |
| `Contents/Resources/glib-2.0/schemas/gschemas.compiled` | compiled by the script, for the same GTK-aborts-without-it reason as on Windows. |
| `Contents/Info.plist` | the minimum Finder needs: what to run, what to call it, who it is — versioned so "Get Info" agrees with the title bar. |
| `README.txt` | beside the `.app` in the zip: the one Gatekeeper sentence, below. |

## The dylibs are derived, not written — and rewriting them is half the job

The walk is the Windows one with `otool -L` for `objdump`: copy every library
the binary imports from the Homebrew prefix, repeat over each copy's imports,
and treat whatever falls outside the prefix as a system library every Mac has.
An `@rpath` or `@loader_path` reference is resolved beside its referencing
dylib and then in brew's lib directory — the real case arrived on the first
build: libwebp naming its sibling libsharpyuv through the rpath Homebrew
points at the package's own lib dir — and one that resolves nowhere fails the
build loudly.

Copying is only half of it, and this is where macOS differs from Windows in
kind rather than spelling. A Windows binary imports DLLs *by name* and finds
them beside itself; a Mach-O file records each dependency's **absolute
path**. A bundle of copied dylibs still pointing at `/opt/homebrew/...` works
on the machine that built it and on no other — silently, the same
stale-bundle failure the DLL section describes, arriving through a different
door. So every reference is rewritten to
`@executable_path/../Frameworks/<name>` with `install_name_tool`, in the
binary and in every copied dylib, and the standalone check re-walks the
finished bundle and fails on any reference that still names the Homebrew
prefix — the one miss the smoke test cannot catch, because on the build
machine the absolute path still resolves.

## The launcher, the signature, and the Gatekeeper sentence

The launcher exists for the `.cmd`'s reason with a macOS cause: GLib finds
its compiled GSettings schemas through the prefix it was built for — the
build machine's Homebrew, not this bundle — so
`Contents/MacOS/ferrocommander` exports `GSETTINGS_SCHEMA_DIR` into the
bundle and `exec`s the binary. Without it, GTK aborts at startup on every
Mac except the one that built the zip.

Signing is not optional on Apple Silicon: the OS refuses unsigned Mach-O
outright, and `install_name_tool` invalidates the ad-hoc signatures the
linker left. So the script re-signs everything it rewrote (`codesign -s -`)
— free, no account, and enough to *run*. What ad-hoc does not buy is
Gatekeeper's blessing on a downloaded zip: the first start needs
right-click → Open, which is the one sentence `README.txt` exists to say.
Notarization (an Apple Developer account, certificate secrets, `notarytool`)
buys the double-click and is deliberately not spent yet.

## What the macOS script checks

The same three as the other two, in this platform's forms: **nothing
declared** — the re-walk above, whose FAIL means a missed rewrite, not a
missing file; **everything is in there** — binary, launcher, plist, schemas,
notes, by path; **it starts, and stays started** — the same
twenty-second `timeout` verdict as Windows, through coreutils' `gtimeout`
because stock macOS ships no `timeout` at all. Run #24 answered the one
question nothing could answer from a distance: a GitHub macOS runner *can*
start a GTK4 window, under GTK's default renderer — no macOS analogue of the
`GSK_RENDERER=cairo` workaround was needed.

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
image has what the `apt-get` line assumes, and whether `fetch-depth: 0` really
gives `build.rs` the commit count. Those were watched on the first real runs.

## The Windows job, and how little of it could be checked

A second job on `windows-2025`: MSYS2 with the MINGW64 toolchain and GTK4, the
`x86_64-pc-windows-gnu` toolchain installed over the runner's MSVC default,
`scripts/package-windows.sh`, and an upload.

Three choices in it are not obvious:

- **`needs: build`, so it runs after the gate rather than beside it.** The
  gate cannot run on Windows — the end-to-end suite drives the binary through
  `Xvfb` and `xdotool`, and MSYS2's GTK4 is a Win32-backend build with no X11
  backend to point at one ([windows.md](windows.md)) — so the Linux job is the
  only thing that verifies the commit *either* package is built from. A binary
  built from an unverified commit is a binary of unknown quality, and waiting
  the extra ten minutes is the whole price of not publishing one.
- **Upload only, never `create`.** The release is the Linux job's: it created
  it or moved it onto this commit, and this only adds an asset. So there is no
  second `create` racing the first, and nothing in this job can leave the
  download URL in the README pointing at nothing.
- **`msys2/setup-msys2` pinned to a commit, not to `v2`.** This job holds
  `contents: write` on every push to the default branch, which is what makes
  the workflow the most attractive thing in the repository to whoever
  compromises a dependency — and a floating tag on a third-party action is
  exactly that door. The comment beside the hash says which release it is, so
  the next person can tell an upgrade from a substitution. The action is used
  at all because the alternative — `pacman` against whatever MSYS2 the runner
  image happens to carry — inherits the update dance `windows.md` opens with,
  on a machine nobody can watch.

**What could be checked before pushing, and was:** the file parses; all seven
`run:` blocks are valid shell (`bash -n`); `scripts/package-windows.sh` is
`100755` in the index, which a workflow otherwise discovers by failing; and
the launcher the script generates was run on a real Windows machine from a
different working directory, where `%~dp0` resolved to the folder beside it
and `GSK_RENDERER=cairo` was observed arriving in the child process.

**What could not be checked is the build itself.** It was written on a Windows
machine with neither Rust nor MSYS2 installed, so nothing downstream of
`cargo build` has run once: not the link against GTK4, not the `objdump` walk,
not the schema compile. The step most likely to need a second attempt is the
smoke test, because it needs a GitHub runner to be able to start a GUI process
at all — and if it cannot, that is a property of the runner, not of the
bundle, and the check has to move to the laptop rather than be deleted.

This is the same position [the Ubuntu job](#the-workflow-and-what-could-be-checked-about-it-before-it-ran)
was in before its first green run, which is the argument for the script/workflow
split rather than an exception to it.

The release step itself is no longer a question, because it stopped being a
delete. It is `gh release create … || gh release edit …`, then
`gh release upload --clobber`: the release is *moved* to the new commit rather
than destroyed and rebuilt, so there is no window in which the download URL
in the README points at nothing.

## The macOS job, and what its first four runs each taught

A third job on `macos-15` — pinned, and arm64, so the build is native. After
the gate for the Windows job's reason: the end-to-end suite is X11-only, so
the Linux job is the only thing that verifies the commit any package is built
from. `brew install gtk4 coreutils`, `rustup update stable`,
`scripts/package-macos.sh`, an upload — and every line of that install step
is a scar, not a guess:

- **`rustup update stable`** because the image's installed stable lags the
  channel, and `rust-toolchain.toml`'s `stable` resolves to what is
  *installed* (run #19: "rustc 1.97.1 is not supported"). The third variation
  on one lesson in three runner setups: the toolchain a job gets is decided
  by the image, not by the toolchain file's word.
- **`coreutils`** because it is not on the image and stock macOS has no
  `timeout` (run #23) — caught by the script's own resolve-early guard at
  second three rather than minute twenty.
- The `@rpath` resolution (run #21) and a missing `mkdir` for the schemas
  (run #22) were the script's two rounds; runs #21–#24 were iterated with
  the workflow temporarily cut down to this one job, at the owner's
  direction, and the cut reverted the moment it went green.

## The tag is called `rolling`, not `main`

A tag and a branch with the same name make every `git push origin main` fail
with `src refspec main matches more than one`: git cannot tell which of the
two refs is meant, and the push has to name `refs/heads/main` in full. That
is a foot-gun on every push for the life of the repository, in exchange for a
tag name nobody reads — the release *title* is what people see, and the
download URL is what they use.

Found by pushing, not by reading: the workflow published a tag called `main`
on its first green run, and the next `git push origin main` failed.

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
