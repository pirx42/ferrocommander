# macOS groundwork — everything a Mac is not needed for

Status: Implemented

`future-improvements.md` prices macOS as four pieces of work and closes with
"verify Windows first". Windows was verified on 2026-08-30
([windows.md](../../windows.md)), so this is the next step: **do every part that
a Linux box can do and check**, so that when a Mac exists the remaining work
is fixes, not architecture.

## 0. What was decided, and by whom

Owner, 2026-08-30, on the four questions the proposal raised:

- **The gate gains a macOS cross-check**, `aarch64-apple-darwin` only.
- **`mount_points` uses `getfsstat(2)` via `libc`** — the real API, not a
  parse of `mount`'s output. *(The proposal called `libc` a new dependency
  and priced the decision accordingly; it is not — `space()` has used
  `libc::statvfs` since the status line gained its disk figure. The decision
  stands, one input to it corrected.)*
- **The macOS keymap layer is prepared as data**, shipped dormant, marked
  unjudged until a Mac can feel it.
- **No packaging script.** A bundler nobody can run is a sequence nobody can
  check — the repository's own argument, applied to itself. The bundle's
  shape is recorded in prose instead.

## 1. What is true today — measured, not recalled

Three facts were established before this plan was written, and each moved it:

- **`tc-core` is already clippy-clean for `aarch64-apple-darwin`.** The
  target installs with one `rustup` command and the check runs in **7.6 s**
  warm — every dependency compiles, `trash` and `notify` included. So the
  engine "nearly compiles for macOS" understates it: it compiles today.
- **Which means the three Linux assumptions are invisible to a compiler.**
  `mount_points` reads `/proc/self/mounts`, `config_dir` follows XDG,
  `is_hidden` knows only the leading dot — all inside `#[cfg(unix)]`, which
  macOS *is*. They compile for a Mac and answer wrongly on one. The gate
  check pins compile drift in the branches this plan adds; it cannot see
  these until the branches exist. Runtime wrongness needs tests, not cfg.
- **`Cmd` cannot be written in a `[keys]` line at all.** `MODIFIER_NAMES` is
  three entries — ctrl, shift, alt — and `RELEVANT_MODIFIERS` masks
  everything else out **before the lookup**, so even a parsed Cmd binding
  could never fire. GTK on macOS reports the Command key as `META_MASK`.
  This is the one genuinely architectural gap found, and it is exactly what
  the owner asked this plan to surface: without it, "ship a Cmd layer later"
  was quietly impossible.

One non-gap, recorded so nobody re-checks it: the command line's
`$SHELL -c` / `/bin/sh` fallback — the thing that runs nothing on Windows —
is fine on macOS, which ships both.

- **`std::os::macos::fs::MetadataExt::st_flags()` exists**, verified by a
  probe compile against the target. `UF_HIDDEN` is reachable from `std`; the
  `libc` dependency is for `getfsstat` alone.

## 2. What this plan deliberately cannot do

Stated up front, because "prepared without a Mac" has a boundary:

- **Nothing here is ever *run* on macOS.** Compile-checked, unit-tested
  against fixtures, and reviewed — the same epistemic level the Windows
  branch had before 2026-08-30, and that day found two real gaps and a
  renderer crash that no amount of cross-checking had seen. Expect the same
  class of surprise; the point of this plan is to make them *small*.
- **`tc-app` stays unchecked for the target**, as it is for Windows: GTK's
  `-sys` build scripts need a macOS libgtk-4 via pkg-config that a Linux box
  cannot provide. Every platform-divergent line stays in `tc-core`, which is
  what makes that boundary acceptable ([vfs.md](../../vfs.md)).
- **The end-to-end driver is out of scope.** Xvfb and xdotool are X11-only;
  the macOS equivalent needs a real GUI session. `future-improvements.md`
  keeps that entry, unchanged: it is the part that decides whether a macOS
  release is honest, and it cannot be started here.
- **No `.app` bundle script** (§ 0). The shape it will need — the
  `Contents/MacOS` layout, the GTK dylibs carried inside, the unanswered
  codesigning/notarization question — goes into `future-improvements.md` as
  prose.

## 3. Phases — one phase, one commit

**Phase 0 — coverage pre-check** (skill 43). What pins platform behaviour
today: `parse_mount_table` is fixture-tested against a made-up machine, and
the mount *judge* (pseudo types, pseudo roots) is separate from the reader —
the split this plan leans on. `config.rs` and the hidden-flag path have
headless tests. The cross-check baseline is green (§ 1), so anything it
reports after a phase is that phase's doing.

**Phase 1 — the gate check.** `aarch64-apple-darwin` beside the Windows step
in `green-gate.sh`; the `rustup target add` line in CLAUDE.md's
prerequisites next to the Windows one. Lands first so every later phase is
checked by it from the moment it exists.

**Phase 2 — `vfs/platform.rs` grows its macOS branches.**
- `mount_points`: a `getfsstat` reader (via `libc`, workspace-managed),
  feeding the **same judge** the Linux reader feeds. The judge's fixture
  tests stay shared; the new reader gets its own unit coverage for the
  statfs-record → candidate translation, with the record shape as a fixture.
- `config_dir`: `~/Library/Application Support/ferrocommander`, with
  `XDG_CONFIG_HOME` still honoured when set — the rule extracted pure so it
  is testable on Linux.
- `is_hidden`: leading dot **or** `UF_HIDDEN` in `st_flags` (a named
  constant; the flag's value is stable BSD). The rule extracted pure —
  `hidden_from(name, flags)` — and tested on Linux; only the `Metadata` read
  is cfg'd.
- Everything else stays in the shared `unix` branch, which § 1 shows already
  compiles. Where a function splits, it splits `target_os = "linux"` /
  `target_os = "macos"` *inside* `cfg(unix)`, keeping "every platform
  difference in one file" true.

**Phase 3 — `Cmd` becomes expressible, then the layer.** Two commits:
- `MODIFIER_NAMES` gains `("cmd", META_MASK)` and `RELEVANT_MODIFIERS`
  includes META, with headless tests: `"cmd+c"` parses, resolves, and
  round-trips through `key_spec`; a Cmd binding fires under META and not
  under Ctrl. All testable on Linux — the keymap is pure.
- The macOS default layer as data: the Cmd-translated table, applied between
  the defaults and the user's `[keys]` under `cfg(target_os = "macos")`,
  and tested on Linux by applying it *explicitly* — the layer mechanism is
  what the user-override tests already exercise. Documented as unjudged:
  which bindings move to Cmd is a taste decision a Mac has to confirm.

**Phase 4 — target-honest engine tests.** The Windows run found six engine
tests asserting Linux rather than the engine
([future-improvements.md](../../future-improvements.md)); the same sweep run
for macOS assumptions — paths, `/proc`, flag semantics — so a future
`cargo test` on a Mac fails only where macOS genuinely differs.

**Phase 5 — the documentation, in the same commits as their phases where
possible, plus the sweep:** `vfs.md`'s platform table gains a macOS column;
`future-improvements.md`'s macOS section shrinks to what genuinely remains
(the e2e driver, the bundle, the feel-tuning) and records what moved here;
CLAUDE.md's build section gains the target-add line.

**Phase 6 — refactoring audit** (skill 49). **Done.** One blanket
`#[allow(unused_mut)]` became the precise
`cfg_attr(not(macos), allow(unused_mut))` with its reason; nothing else asked
for changing. The `apply()` refactor in phase 3 had already removed the one
duplication the work created, and the mount judge extraction *reduced* the
line count it touched.

## 6. What the doing taught that the plan did not know

- **The cross-check paid for itself four times before the plan closed**: dead
  Linux code under a macOS build (twice, symmetric), and a clippy lint inside
  `cfg(not(linux))` that the host cannot even compile.
- **The sharpest find was not on the list.** The trash suite redirected the
  freedesktop trash through `XDG_DATA_HOME`; macOS ignores XDG, so on a Mac
  the suite would have trashed its fixtures into the account's *real* bin —
  the E2E harness's old Linux bug, met from the other side. It is
  `target_os = "linux"` now, for the layout it actually asserts.
- **One engine fix fell out for every platform**: `remove_file` on a
  directory now answers `IsADirectory` everywhere, settling a row of the
  Windows six as engine behaviour rather than as three test skips.
- **The libc premise was wrong** (§ 0's correction): the dependency this plan
  "added" had been there since the disk figure.
- **And the plan itself falsified a documented claim without noticing.** The
  Cmd layer put three `cfg` markers into `tc-app`'s `keymap.rs` while
  `vfs.md` still said `tc-app` contains no `cfg` at all — caught only when
  the owner asked, after the audit, whether conditionals had spread. The
  sentence now counts its one exception and names the two lines no gate can
  reach. A plan whose whole subject was documentation-checked platform
  boundaries drifted a platform-boundary document mid-flight; the drift
  review's lesson, demonstrated on its own author.

### § 5 addendum — the first-run review list, final

The `platform.rs` macOS lists and `getfsstat` reader; the `remove_file`
EPERM branch (unreachable on Linux — measured, `unlink` answers `EISDIR`
even under a read-only parent); GTK delivering Command as `META_MASK`; the
watcher's kqueue behaviour on reads, which the inotify-shaped watch tests
assume; and the config-path convention.

## 4. Effort

Corrected per skill 45, factor 0.10.

| Phase | Raw | Factor | Corrected |
|---|---|---|---|
| 0 pre-check | 0.25 d | 0.10 | 0.25 h |
| 1 gate check | 0.25 d | 0.10 | 0.25 h |
| 2 platform branches | 2 d | 0.10 | 2 h |
| 3 Cmd + layer | 1.5 d | 0.10 | 1.5 h |
| 4 target-honest tests | 1 d | 0.10 | 1 h |
| 5 docs | — | — | 0.5 h |
| 6 audit | 0.5 d | 0.10 | 0.5 h |

**About six hours**, gate runs included at one per phase.

## 5. What would make this wrong

Recorded so the day a Mac arrives, the checking starts here:

- `getfsstat`'s record layout is asserted from documentation, not from a
  running system. The reader's fixture is hand-built; a real Mac's first
  `mount_points()` call is the test that counts.
- GTK-on-macOS delivering Command as `META_MASK` is documented behaviour,
  unverified here. If it arrives as something else, phase 3's layer is data
  and moves in minutes — that is the point of it being data.
- `~/Library/Application Support` versus XDG is a convention call; a Mac
  user who disagrees edits one pure function.
- And the standing one: the Windows day found its real bugs at the layer no
  cross-check reaches. Assume the same here.
