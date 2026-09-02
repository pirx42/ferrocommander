# Windows — building it, running it, and testing it

← Parent: [CLAUDE.md](CLAUDE.md)

Windows is a supported target ([vfs.md](vfs.md)), and until 2026-08-30 nothing
had ever been *run* there. Doing it the first time cost most of a session, and
almost none of that went on the code. This file is what that session would
have needed to know at the start.

The short version: the build is clean and the app works, with one environment
variable. Two things that look like missing dependencies are not, and the
end-to-end suite cannot run here at all.

## The toolchain

**MSYS2's MINGW64 environment, not UCRT64.** Rust's `x86_64-pc-windows-gnu`
links msvcrt, and the `mingw-w64-x86_64-*` packages are the ones that match
it; the UCRT64 packages do not, and the mismatch surfaces at link time rather
than at install time, which is the expensive way to find out.

```powershell
winget install --id MSYS2.MSYS2 -e
C:\msys64\usr\bin\bash.exe -lc "pacman -Syuu --noconfirm"
C:\msys64\usr\bin\bash.exe -lc "pacman -S --needed --noconfirm mingw-w64-x86_64-toolchain mingw-w64-x86_64-gtk4"
```

The first `pacman -Syuu` updates `msys2-runtime` and then **kills its own
shell** — that is normal, not a failure. Run it again before installing
anything.

**Rust has to be told to use the gnu host.** The installer defaults to
`x86_64-pc-windows-msvc`, which cannot link against the mingw GTK4:

```powershell
winget install --id Rustlang.Rustup -e --override "-y --default-host x86_64-pc-windows-gnu"
rustup target add x86_64-pc-windows-gnu   # for the green gate's clippy-windows step
```

Both the build and the finished binary need mingw's `bin` on `PATH` — that is
where the GTK4 DLLs live, so a binary that built fine still will not start
without it:

```powershell
$env:PATH = "C:\msys64\mingw64\bin;$env:USERPROFILE\.cargo\bin;" + $env:PATH
cargo build --workspace --release
```

Measured 2026-08-30: a clean release build of the workspace takes **57 s** and
produces no warnings. GTK4 was 4.22.4, rustc 1.98.0.

## `os error 4551` is not a missing dependency

If a build dies with *"an application control policy has blocked this file"*
on a `build-script-build.exe`, the cause is **Smart App Control**, which
blocks freshly compiled unsigned executables — which is what every Cargo build
script is. It is worth naming here because the error says nothing about
itself, and the obvious reading is that a package is missing:

```powershell
Get-ItemProperty 'HKLM:\SYSTEM\CurrentControlSet\Control\CI\Policy' |
    Select-Object VerifiedAndReputablePolicyState   # 1 = enforcing, 0 = off
```

It is not reliably worked around. The block is not a stable verdict — running
the blocked executable by hand once can let the next attempt through, so a
build appears to advance one crate per run and looks like progress; it is not,
and a thirty-attempt retry loop still did not finish. Turning Smart App
Control off is **irreversible** on Windows 11 (re-enabling needs a Windows
reset), so it is the owner's decision and not one to take on their behalf.

## Line endings will bite twice

Set this before anything else, and do it in a fresh clone:

```powershell
git config core.autocrlf false
```

`autocrlf = true` is the Windows default and it breaks the build in two
different ways, neither of which names the real cause:

- **A checked-out CRLF tree fails a test.** `docs/keymap.md`'s action table is
  compared against a string the code generates ([keymap.md](keymap.md)), and
  CRLF makes them differ. The failure reads as "the docs are out of date"
  and the docs are fine.
- **Turning it off afterwards is not enough.** Files already checked out stay
  CRLF, and `git status` calls them clean, because git compares stat
  information and the cache still matches. 182 of the first 400 tracked files
  sat in that state here while git reported a clean tree. Committing from
  Windows then commits CRLF. Force the rewrite:

  ```bash
  git ls-files -z | xargs -0 rm -f && git checkout -- .
  ```

  Back up any uncommitted work first — that deletes every tracked file before
  restoring it.

Related: `scripts/green-gate.sh` dies on its own `#!/usr/bin/env bash\r`
shebang with a message about `bash\r` that explains nothing.

## Running it: `GSK_RENDERER=cairo`

The app **does not start** under the renderer GTK picks for itself. It exits
about six seconds in with `0xC0000005`, silently, without ever presenting a
window. Under cairo it works: both panes list, the drive bar offers the real
drives, the status line and the command line are live.

```powershell
$env:GSK_RENDERER = "cairo"
.\target\release\ferrocommander.exe
```

Why this is not simply "Vulkan is broken here", and what would be needed to
settle it: [ui-shell.md](ui-shell.md) § *The renderer on Windows*.

## Packaging it for somebody else

`scripts/package-windows.sh`, from a MINGW64 shell. It builds, gathers the
GTK4 DLLs the binary actually imports, compiles the GSettings schemas without
which GTK aborts at startup, writes the launcher that sets the renderer,
checks the bundle is complete, starts the app to see that it survives its
first twenty seconds, and zips what is left.

It needs one package beyond the toolchain above, and needs it only for this:

```powershell
C:\msys64\usr\bin\bash.exe -lc "pacman -S --needed --noconfirm zip"
```

MSYS2's own `zip` rather than a `mingw-w64-x86_64-` one — it is the archiver,
not something the build links against.

The script checks the active Rust toolchain before it starts, and refuses on
the MSVC host rather than letting it fail at link time, which is the failure
this file opens with. The same script runs on every commit to `main` from a
`windows-2025` runner, and its output is published beside the `.deb`.

What the bundle holds, why it holds so little of what a GTK bundle usually
holds, and why there is a `.cmd` beside the `.exe`:
[packaging.md](packaging.md).

## A command runs through `cmd`, and `Enter` runs no command at all

Every command this program runs — the command line, `F4`'s editor, the
compare tool and `Enter` on a file — went through `$SHELL` or `/bin/sh -c`,
which is why none of them did anything on Windows. The interpreter is
`%ComSpec%` with `/C` here and `$SHELL -c` elsewhere.

**That was shipped on 2026-09-01 and did not work**, and the three reasons
are worth keeping, because every one of them is a *composition* bug — a
string that was wrong, visible from Linux, with no Windows machine needed to
see it:

| | Wrong | What Windows saw |
|---|---|---|
| the path | `VfsPath::as_str`, not `to_std_path` | `/C:/Users/pirx/a.exe` — a leading `/`, which `cmd` reads as the start of a switch, and separators the wrong way round |
| the quotes | POSIX `'…'` | `cmd` has no single-quote quoting, so the quotes became part of the filename |
| the argument | `Command::arg` | it quotes for the **C runtime's** rules — wrapping in `"` and escaping inner quotes as `\"` — and `cmd` reads neither. `raw_arg` is the only way to hand `cmd` a line |

So `Enter` on `C:\Users\pirx\a.exe` ran

```
cmd.exe /C "start \"\" '/C:/Users/pirx/a.exe'"
```

and Windows put up a dialog about a path that exists on no machine.

**`Enter` no longer builds a line at all.** Handing a file to the desktop is
not a command, and constructing one only to have an interpreter take it apart
again is where all three bugs came from. It is now one call per platform with
the path as a **single argument**: `xdg-open` or `open` spawned directly, and
on Windows `ShellExecuteW` — the call Explorer itself makes for a
double-click, hand-declared beside `GetDiskFreeSpaceExW` for the same reason.
There is no quoting on that path, so there is none to get wrong. A `.png`
opens in the system viewer, a `.exe` starts; the program starts programs,
which is what a double-click does and what was asked for.

What still builds a line is `F4` with a *configured* editor, the compare tool
and the command line the user typed — those are command lines by definition.
They now carry native paths in `"` quotes on Windows, and reach `cmd`
verbatim.

One sharp edge is left there, and is left on purpose: **`cmd` expands
`%VAR%` inside double quotes**, so a file named `%TEMP%.txt` handed to a
configured editor still goes wrong. `Enter` is immune, because it never
builds a line. See
[future-improvements.md](future-improvements.md).

## The command tests run here, and they are the only ones that do

`cargo test -p fc-core --test command` runs in the `windows` CI job, before
the package is built. It is the answer to a question that had been asked
three rounds running and answered the same way every time — *"the Windows
runner cannot be tested where it runs"* — while three Windows changes shipped
on a twenty-second smoke start, and the third arrived broken in three ways.

The tests are shared, not duplicated: what each one *claims* is the same on
both platforms, and only the wording of the line differs — `ls` / `dir /b`,
`echo a; echo b` / `echo a& echo b`, `true` / `exit 0`. That phrasebook is a
`mod line` at the top of `crates/fc-core/tests/command.rs`, and reading it is
the fastest way to see what `cmd` is: not a poorer `sh` but a different
language, in which `;` does not separate commands, there is no `true`, and a
wildcard is expanded by the *program* rather than the interpreter.

Two of them are about this platform rather than shared by both, and they are
the ones that would have caught the bug: the composed editor line for a path
the VFS holds as `/C:/Users/pirx/a file.txt`, and which quoting style the
build picked.

The job sets `TMP` and `TEMP` to the runner's own temporary directory. MSYS2
hands its shell a POSIX `/tmp`, and the test binary is a *Windows* program:
`tempfile` would ask the operating system for `\tmp` on the current drive,
and every fixture would fail to be created.

## The context menu is Explorer's

A right button held on a row, `Shift+F10` or `Menu` on Windows shows
**the shell's own menu** — the one Explorer shows, with every verb and
every shell extension the machine has — and a right button on the empty
space below the rows shows the folder's background menu, *New ▸* and
*Paste* included. The program's own GTK menu ([ui-shell.md](ui-shell.md))
is what remains for an archive's entries, which have no path on the disk,
and what anything gets when the shell declines.

The conversation with the shell lives in its own crate, `fc-shellmenu`,
and the crate's shape is the point: it has no GTK in it, so
`cargo clippy -p fc-shellmenu --target x86_64-pc-windows-gnu` type-checks
every line from the Linux gate — GTK for Windows cannot be built there,
so a `#[cfg(windows)]` module inside `fc-app` would be checked by nothing
until the Windows job ran — and its tests run on the Windows CI job without
building the application in test mode. What they check is everything up
to the popup: a real temporary file becomes a PIDL, the PIDL's folder hands
out an `IContextMenu` for it, the menu fills an `HMENU`, and *delete* is
among the verbs in it; the same for two files of one folder, and for a
folder's background through its `IShellView`. `TrackPopupMenuEx` and
`InvokeCommand` are the two calls that need a desktop, and no gate
anywhere runs them — **the owner confirmed both on Windows 11 on
2026-09-02**, the menu appearing and its verbs doing what they say. That
is the whole of the evidence for those two, and it is a person rather
than a test: a change to the popup or the invocation is checked by
somebody opening a menu, or it is not checked.

Three details of the plumbing, each the answer to a way this goes wrong:

- **The popup is owned by a hidden window of ours**, not by GTK's. A
  popup's owner receives `WM_INITMENUPOPUP`, `WM_MEASUREITEM`, `WM_DRAWITEM`
  and `WM_MENUCHAR`, and the owner-drawn submenus — *Send to*, *Open with*
  — draw themselves only if those reach `IContextMenu3::HandleMenuMsg2`.
  A window procedure of our own forwards them; subclassing GTK's window for
  the life of a popup would reach into a toolkit's window, which this
  program does not do. The hidden window is made the foreground window
  first, or the popup does not dismiss on a click elsewhere — the
  documented tray-icon dance.
- **The GTK main loop stops while the menu is up.** `TrackPopupMenuEx` is
  modal and pumps its own messages, on the UI thread; a job's progress bar
  holds still for the second the menu is open, as Explorer's own window
  does. A second thread would need foreground-window games that differ
  between Windows versions.
- **The shell hands out menus per folder**, so a branch view (`Ctrl+B`),
  whose marked rows may live in several, asks for the cursor row alone
  rather than pretending with desktop-relative PIDLs.

The `windows` crate is the same crate at the same version `trash` already
pulls in, pinned once in the workspace so the two cannot become two copies.
`gdk4-win32` gives the application its window's `HWND`, for the menu's
placement and for the shell's own dialogs — *Properties* — to be parented
on.

## The free-space figure, and its one Win32 call

The status line's `free of total` was blank on Windows until 2026-09-01:
`statvfs` has no equivalent in `std`, and the branch returned nothing.
`GetDiskFreeSpaceExW` is the call, and it is **hand-declared** — one
`extern "system"` block beside the `libc::statvfs` the Unix branch already
uses — rather than pulled in with a Windows API crate for a single number.

Two details worth keeping: the name is UTF-16 with a terminating NUL, because
the `W` in the entry point is the wide-character form; and the figure taken is
the **first** out parameter, the space available to the calling user, which is
the same distinction the Unix branch draws between `f_bavail` and `f_bfree`.
A zero return is a failure and shows nothing at all — an empty status line is
honest, a `0 B free` is a full disk.

The UTF-16 conversion is tested on the Linux gate. The call itself is not
tested anywhere, for the reason the next section gives.

## `C:\` is a root; `C:` is somewhere else entirely

A native Windows path ending in a drive letter and a colon, with **no**
trailing separator, is *drive-relative*: `C:` means "wherever this process
currently is on drive C", which for a program started from its own folder is
that folder. `C:\` means the root of the drive. One character apart, and the
difference is invisible in every VFS path this program holds — both are
`/C:`.

The mapping trimmed the trailing separator off every native path it built,
which is right for `C:\Users\pirx` and wrong for exactly one path: the drive
root. So going up from `C:\Users` — `..` or `Backspace` — put the pane in
whatever directory the application had been started in. It was there from the
first VFS commit and nothing noticed, because the mapping's own round-trip
test used the home directory, which is several levels down.

**And nothing here could have noticed**, which is the more useful half. The
Windows CI job builds a package and starts it once; it runs no tests. So the
mapping rule now lives outside the `#[cfg(windows)]` module and takes the
separator as an argument, which puts it on the Linux gate — the same shape
`parse_mount_table` uses. What is left that only Windows can run is the
`std::fs` call itself.

## What passes, and what does not

`cargo test -p fc-core --no-fail-fast` — the `--no-fail-fast` matters, because
cargo stops at the first failing test *binary* and hid four of these:

| | Windows | Linux |
|---|---|---|
| `fc-core` | 331 / 346 | 362 / 362 |
| `fc-app` unit tests | 120 / 120 | 120 / 120 |
| end-to-end UI suite | cannot run | see below |

Everything that fails on Windows passes on Linux, so none of it is the engine
being wrong. Nine of the fifteen were one cause — the command line ran
`/bin/sh`, which does not exist here — and **those nine are the ones CI now
runs**, in `cmd`'s own words; the other six are tests that assert Unix
semantics and are written up in
[future-improvements.md](future-improvements.md). The count differs between
the columns because the symlink tests are `#[cfg(unix)]`.

Those figures are from 2026-08-30 and have not been re-measured on a Windows
machine since.

**The UI suite cannot run on Windows at all**, and no package fixes it: the
harness drives the binary through `Xvfb` and `xdotool`, which are X11, and
MSYS2's GTK4 is a Win32-backend build with no X11 backend to point at an X
server even if one were running.

## Running the full suite from a Windows machine

WSL2, which needs one elevated command and a reboot:

```powershell
wsl --install -d Ubuntu     # Administrator
```

Then, inside the distro:

```bash
sudo apt-get install -y build-essential pkg-config libgtk-4-dev xvfb xdotool python3
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
```

Copy the tree into the Linux filesystem rather than building on `/mnt/c` —
9p makes a suite that already takes nine minutes very much worse — and run the
tests as a **non-root** user, or the permission-denial tests pass for the
wrong reason.

Two traps found on 2026-08-30, neither of which is the suite's fault:

- **WSLg mounts `/tmp/.X11-unix` read-only**, so `Xvfb` can never create its
  socket and says so in a message about mode `1777`. `chmod` fails against the
  mount. Turn WSLg off in `%USERPROFILE%\.wslconfig` and `wsl --shutdown`:

  ```ini
  [wsl2]
  guiApplications=false
  ```

- **`xdotool` presses `Alt` that nobody asked for.** On Ubuntu 26.04,
  `xdotool key --clearmodifiers F5` puts `Alt_L` down first and F5 arrives
  with `state 0x8`, so the app reads `Alt+F5` and packs instead of copying.
  107 of 166 UI tests fail, and every one looked at failed this way — the
  cause of the whole 107 was not checked one by one. It is the keymap —
  `keycode 71 = F5 F5 F5 F5 F5 F5 XF86Switch_VT_5`, seven levels — against an
  `xdotool` whose last release was 2016. `setxkbmap` and `xmodmap` do not
  change it, and the GSK renderer is not involved: cairo and the default fail
  identically. Not yet fixed; it would want the harness to normalise the
  keymap after starting `Xvfb`, or a different tool than `xdotool`.

Everything else is green there — `fmt`, both `clippy` steps, the release
build, and the link check.
