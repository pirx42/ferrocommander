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

## What passes, and what does not

`cargo test -p tc-core --no-fail-fast` — the `--no-fail-fast` matters, because
cargo stops at the first failing test *binary* and hid four of these:

| | Windows | Linux |
|---|---|---|
| `tc-core` | 331 / 346 | 362 / 362 |
| `tc-app` unit tests | 120 / 120 | 120 / 120 |
| end-to-end UI suite | cannot run | see below |

Everything that fails on Windows passes on Linux, so none of it is the engine
being wrong. Nine of the fifteen are one cause — the command line runs
`/bin/sh`, which does not exist here — and the other six are tests that assert
Unix semantics. Both are written up in
[future-improvements.md](future-improvements.md); the count differs between
the columns because the symlink tests are `#[cfg(unix)]`.

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
