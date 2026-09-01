# Enter on Windows — the quoting, and the end of "cannot be tested here"

Status: **Done**, 2026-09-01 — the fix and the CI step that makes it
checkable, each behind a green gate. Outcome in § 6.

One finding, and one question that matters more than the finding:

> did you test at least that calling an executable file works by pressing
> return inside ferrocommander? on windows it does not work. there appears a
> dialog that `//` (or similar) can not be found

**No — and that is the actual defect.** The last round shipped a Windows
code path that no gate anywhere runs, said so in its own plan (§ 6, *"the
Windows runner cannot be tested where it runs — third time in three rounds"*),
and shipped it anyway. Writing that down was not a mitigation; it was a
prediction, and it came true one commit later. This round fixes the bug **and
removes the excuse**: the Windows CI job starts running the command tests, so
the next line that does not work on Windows fails on the machine that can
tell.

## 1. What is actually wrong

`open_with` composes the line and hands it to `run`:

```rust
let line = format!("{program} {}", shell_quoted(path.as_str()));   // command.rs
fn shell_quoted(text: &str) -> String { format!("'{}'", …) }        // POSIX quoting
Command::new(&shell).arg("/C").arg(line)                            // run()
```

Three separate things are wrong with that on Windows, and each alone breaks it:

| # | Wrong | What Windows sees |
|---|---|---|
| 1 | `path.as_str()` is the **VFS** path, not the native one | `/C:/Users/pirx/a.exe` — leading slash, forward slashes. `cmd` reads a leading `/` as the start of a *switch* |
| 2 | `shell_quoted` quotes POSIX-style, with `'` | `'…'` — `cmd` has no single-quote quoting at all, so the quotes become part of the filename |
| 3 | `Command::arg` applies **MSVCRT** argument quoting to a line meant for `cmd` | the whole line is re-wrapped in `"` and its inner quotes escaped as `\"`, which `cmd` does not understand |

So `Enter` on `C:\Users\pirx\a.exe` runs

```
cmd.exe /C "start \"\" '/C:/Users/pirx/a.exe'"
```

and Windows opens the dialog the report describes, because that is not a path
on any machine. `to_std_path` — the function that turns a VFS path into
`C:\Users\pirx\a.exe`, tested since 2026-09-01 — was right there and was not
called.

It is worth naming that all three are *composition* bugs, not Windows
mysteries: every one of them is visible in a string, from Linux, in a unit
test. Nothing here needed a Windows machine to catch. It needed a test.

## 2. Decisions

1. **`Enter` stops going through a shell at all.** Handing a path to the
   desktop is not a command line, and building one only to have an
   interpreter take it apart again is where all three bugs came from. It
   becomes a direct call per platform: `xdg-open`/`open` spawned with the
   path as **one argument** — no quoting exists to get wrong — and on Windows
   `ShellExecuteW`, which *is* the API a double-click uses. Same
   hand-declared `extern "system"` treatment as `GetDiskFreeSpaceExW`, for
   the same reason: one entry point does not earn a dependency.
2. **The three opener constants go away.** `start ""`, `xdg-open` and `open`
   existed only to be pasted into a shell line. Two survive as bare program
   names; `start` disappears entirely, and with it the empty-title detail
   that needed a test to defend it.
3. **An empty editor line means "the desktop's own"**, which is what an
   unset setting already meant — so `F4` with nothing configured and `Enter`
   are now one code path rather than two that agree by construction.
   `DEFAULT_EDITOR` becomes that empty line.
4. **The shell runner is still fixed**, because `F4` with a real editor, the
   compare tool and the typed command line all still go through it: native
   paths, `cmd` quoting on Windows, and `raw_arg` so `cmd` gets the line
   verbatim.
5. **Quoting is a parameter, not a `cfg`.** `quoted(text, Quoting::Cmd)` is
   checkable from Linux; `#[cfg(windows)] fn quoted` is not. Same trick as
   `windows_native(path, separator)`, and the reason it exists.
6. **The Windows CI job runs `cargo test -p fc-core --test command`.** The
   command tests get a per-platform phrasebook — `ls` / `dir /b`,
   `echo a; echo b` / `echo a& echo b` — so the *claims* are shared and only
   the wording differs. Costs a couple of minutes on a job that already
   builds the release binary, and buys the one thing this round is missing.

## 3. Phases

| # | | Commit |
|---|---|---|
| 1 | this plan | docs |
| 2 | the runner and the opener, with the docs they change | fix(command) |
| 3 | the Windows job runs the command tests | ci |
| 4 | audit | refactor |

Phases 2 and 3 of the list above were written as separate commits and landed
as one: dropping `WINDOWS_OPENER` is what forces `Enter` off the shell, and a
commit that fixed the quoting of a line the next commit deletes would be a
commit of something nobody ever ran.

## 4. Effort

Corrected per skill 45 (factor 0.10, holding across the last five plans).

| Phase | Raw | Corrected |
|---|---|---|
| 2 the runner | 1 d | 1 h |
| 3 the opener | 1 d | 1 h |
| 4 Windows CI tests | 1.5 d | 1.5 h |
| 5 docs + audit | 0.5 d | 0.5 h |

About four hours, plus a full-gate run (~15 min) per phase commit.

## 5. What would make this wrong

- **Phase 4 is the only part that changes anything.** Phases 2 and 3 are a
  better-written version of code that was already wrong once; if the Windows
  job does not actually run these tests, this round has the same standing as
  the last one — a claim.
- **`ShellExecuteW` may want COM.** The documentation says a Shell extension
  may require a single-threaded apartment, so the call gets its own thread
  and `CoInitializeEx` on it. Untestable from here, and named as such.
- **`cmd` expands `%VAR%` inside quotes**, so a file named `%TEMP%.txt`
  handed to `F4`'s configured editor still goes wrong. `Enter` is immune
  after phase 3 because it never builds a line. Recorded in
  [future-improvements.md](../../future-improvements.md) rather than papered over.
- **Wildcards are not the shell's on Windows.** `dir /b *.txt` works because
  `dir` expands the pattern, not because `cmd` did — so the phrasebook's
  glob test asserts the pipe, which both platforms really share.

## 6. Outcome

Three commits. `bf9e65a` is the fix, `4c2e642` the CI step, and this one the
audit.

**What the fix came to.** `Enter` builds nothing: `open_in_desktop` hands the
path over as one argument, to `xdg-open`, to `open`, or to `ShellExecuteW`.
`WINDOWS_OPENER` and `DESKTOP_OPENER`'s public form are gone, `DEFAULT_EDITOR`
is the empty line, and an unconfigured `F4` is now literally the same call as
`Enter` rather than a second route to the same idea. What still composes a
line — a configured editor, the compare tool, a typed command — does it
through `editor_line`/`substituted`, both of which take the quoting style as
an argument and are checked in both styles from Linux.

**What is now verified, and by what.** The `windows` job runs 16 command
tests, of which 14 are shared claims in `cmd`'s own words and two are about
the platform:

| Verified on Windows by CI | Not verified anywhere |
|---|---|
| a typed line runs, in the pane's directory, and its output and exit code come back | `ShellExecuteW` actually launching a handler — it needs a desktop session |
| both streams, truncation, a missing program | `xdg-open`/`open` spawning, for the same reason |
| a pipe, and a wildcard the program expands | that a `.exe` starts and a `.png` opens |
| which quoting style the build picked | |
| the editor line composed from a VFS path — the bug, as a string | |

That second column is honest rather than resigned: what a handler does with a
file is the desktop's answer and not something a test can assert without a
desktop. What *can* be checked without one — every string this program builds
— now is.

**The lesson, which is the reason this plan exists.** The last round wrote
*"the Windows runner cannot be tested where it runs"* in its own § 6 and
shipped anyway; one commit later it was broken in three ways, all three of
them a string that a Linux test could have read. Writing a limitation down is
not a mitigation. The two useful moves were both cheap: make the
platform-dependent decision a *parameter* so the gate that runs can see both
answers, and split the composition out of the function that spawns, so there
is a value to assert on at all.

## 7. What the audit changed

- `opening_line` had one caller and existed only as a step; it is now the
  `format!` inside `editor_line`, which is the function that is the claim.
- A doc comment on `push_line` illustrated the quoting bug with `start ""` —
  a constant the same commit deletes.

Nothing else. The change is one function per platform and one enum; there is
no second place any of it is done.
