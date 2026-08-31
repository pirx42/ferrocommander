# A macOS package (Apple Silicon), built and checked on every commit

Status: In Progress — decisions 1–3 settled by the owner, 2026-08-31

The third and last platform job: what `package-windows.sh` and the `windows`
job did for Windows, done for macOS on GitHub's arm64 runners. The macOS
groundwork ([2026-08-30 plan](archive/2026-08-30-macos-groundwork.md))
stopped at "everything possible without a Mac"; a `macos-15` runner **is** a
Mac, so this plan is also the first time ferrocommander will ever run on one.
The smoke test alone retires the biggest unknown on the groundwork plan's § 5
first-run review list — or reports it, which is worth as much.

## 1. What is different from Windows, and what is the same

Same by construction: the version rule (`scripts/version.sh`), the
script-not-workflow rule (a workflow can only be tested by pushing it), the
three checks (nothing declared, nothing installed, starts and stays started),
the rolling release the artifact lands in, and the job order — after the
Linux gate, never beside it, because the gate cannot run on macOS and a
package from an unverified commit is a package of unknown quality.

Different, each a real decision or a real unknown:

- **Native, not cross.** `macos-15` runners are arm64 Macs; the build is a
  plain `cargo build --release` with host `aarch64-apple-darwin` — no MSYS2
  analogue of the default-host trap, because the runner's rustup host *is*
  the target. The host guard stays anyway; it costs three lines and run #17
  showed what it is worth.
- **GTK4 from Homebrew** (`brew install gtk4`), the platform's MSYS2.
- **Dylibs, not DLLs.** The walk is `otool -L` instead of `objdump -p`, and
  it does not end at copying: Homebrew dylibs carry absolute install names
  (`/opt/homebrew/...`), so a relocatable bundle needs `install_name_tool`
  rewrites to `@executable_path/../Frameworks/...` on the binary *and* on
  every copied dylib's own references. Copying without rewriting produces a
  bundle that works on the build machine and nowhere else — the exact
  silent-staleness failure the Windows script's comments warn about, in its
  macOS spelling.
- **Signing is not optional.** arm64 macOS refuses unsigned binaries
  outright, and `install_name_tool` invalidates the ad-hoc signature the
  linker put there — so the script must re-sign (`codesign -s -`) after the
  rewrites, deepest-first, or the smoke test dies with `Killed: 9` and says
  nothing about why. Ad-hoc is free and enough to *run*; what it does not
  buy is Gatekeeper's blessing on download (§ 2, decision 2).
- **BSD userland.** Stock macOS ships bash 3.2, BSD `sed`, and **no
  `timeout`** — the smoke test's verdict mechanism. The runner has coreutils
  (`gtimeout`) preinstalled via Homebrew; the script resolves
  `timeout`/`gtimeout` explicitly rather than discovering the difference as
  a `command not found` twenty minutes into a run.

## 2. Decisions before phase 1

1. **Bundle shape: a minimal `.app`** (recommended) — `Contents/MacOS/` for
   the binary, `Contents/Frameworks/` for the dylibs, `Contents/Resources/`
   for the compiled schemas, a hand-written `Info.plist`. The
   future-improvements entry set the bar at "the standard a Mac user would
   accept", and a bare folder of dylibs is not it; the `.plist` is ~15 lines
   and the layout costs nothing extra since the dylib rewrite has to pick
   *some* relative home anyway. Alternative: a flat folder like the Windows
   zip — cheaper by one plist, and reads as unfinished to its audience.
2. **Signing: ad-hoc + a README sentence** (recommended). The zip arrives
   quarantined either way without notarization; a right-click → Open is the
   documented, honest v1. Alternative: Developer ID + notarization — needs
   an Apple Developer account (99 USD/yr), certificate secrets in the repo
   settings, and `notarytool` plumbing; worth revisiting only if real Mac
   users appear.
3. **Iteration route: on main, like Windows** — the owner's call, against
   the plan's recommendation of a temporary topic-branch trigger, and the
   simpler one: the workflow file stays exactly as merged, with no trigger
   line and no guards to remember to remove. The accepted cost is what the
   Windows job paid — red runs on main until the new job is green, three in
   that case — bounded by the fact that a red `macos` job never blocks the
   `build` job it depends on, so the gate, the `.deb` and the Windows zip
   keep publishing throughout.

## 3. Phases

**Phase 0 — what exists, and what can be checked from here.** No unit tests
cover the packaging scripts; their coverage *is* their self-checks plus CI.
From this Linux box: the workflow parses, every run block is valid shell,
`bash -n` on the script, the link check. Everything downstream of `brew` is
unreachable — the same honesty line the Windows commit drew, which is why
phase 3 exists as its own phase rather than as optimism.

**Phase 1 — the workflow's `macos` job.** `needs: build`, `runs-on:
macos-15` (pinned — `latest` moves, and the day it moves it decides a
different Xcode, brew, and OS under a bundle this path-sensitive),
`brew install gtk4`, then `./scripts/package-macos.sh`, then the
upload-only release step copied from the Windows job. Committed together
with phase 2's script — a job that calls a script that does not exist is
not a phase.

**Phase 2 — `scripts/package-macos.sh`.** The Windows script's structure,
translated where § 1 says it must be: version from `version.sh`; host guard
(`aarch64-apple-darwin`); `cargo build --release --locked`; stage the
`.app`; breadth-first `otool -L` walk copying every dylib under the brew
prefix (a dylib outside it is a system library — `/usr/lib`, frameworks —
which every Mac has and none may ship); `install_name_tool` rewrites;
`glib-compile-schemas` into `Contents/Resources`; ad-hoc `codesign`,
dylibs first, the bundle last; the three checks; the smoke test through
`gtimeout`-or-`timeout`, status 124 the only pass. Zip named
`ferrocommander-macos-arm64.zip`, versionless for the rolling URL, the
versioned name on the `.app`'s inner folder as the Windows zip does it.
No renderer override until the smoke test demands one: Windows earned its
`GSK_RENDERER=cairo` from a measured crash, and macOS starts with GTK's
default and the same instrument pointed at it.

**Phase 3 — merge to main and iterate there until the job is green**
(decision 3). The phase that cannot be planned, only budgeted (§ 4). The known suspects, in
the order they would fire: brew's gtk4 formula missing something the MSYS2
package had; the dylib walk finding `@rpath` entries that need resolving
against the binary's rpaths rather than copying verbatim; codesign order;
and the one genuine coin-flip — whether GTK4 can reach the runner's
WindowServer for the smoke test. If it cannot, the smoke test does **not**
get quietly weakened to "it launched": the failure is recorded in
packaging.md as the macOS analogue of the Windows renderer crash, the
check keeps failing loudly, and the phase ends with the owner deciding
between shipping unchecked (stated in the release notes) or not shipping —
a package whose one honest check cannot run is a product decision, not a
script's.

**Phase 4 — docs, in the same commits as what they describe.**
packaging.md gains the macOS section (bundle anatomy, the install-name
story, the quarantine sentence); the README gains the download line beside
the Windows one; future-improvements.md's macOS table loses its packaging
row and says what the first real run settled of the § 5 review list.

**Phase 5 — refactoring audit (skill 49).** Three packaging scripts now
exist; read them as one diff. The known candidate: the smoke test
(`timeout`-status-124 plus log handling) is about to exist twice — decide
shared-helper versus read-alone with the same argument
`check-page-scroll.sh` already had, and record the verdict either way.

## 4. Effort

Corrected per skill 45; factor 0.10 held exactly on the groundwork plan.

| Phase | Raw | Corrected |
|---|---|---|
| 0 pre-check | 0.25 d | 0.25 h |
| 1+2 job + script | 2 d | 2 h |
| 3 CI iteration | — | wall-clock bound: each round is a push plus a ~20–30 min run (brew, build, smoke); budget 3–6 rounds going by the Windows job's three |
| 4 docs | 0.5 d | 0.5 h |
| 5 audit | 0.5 d | 0.5 h |

Working time about three hours; elapsed time is owned by phase 3's runner
queue, not by the writing.

## 5. What would make this wrong

- **The WindowServer question** decides whether the strongest check can run
  at all, and nothing but the first push answers it. Everything else in this
  plan survives either answer; the *value* of the plan is halved by a "no".
- The dylib walk is asserted from `otool`'s documented output format and
  brew's documented prefix, both unverified here — the same
  documentation-not-measurement caveat the groundwork plan recorded for
  `getfsstat`, and it was right to record it.
- GTK4's macOS backend may want resources the grep-based "no icons, no
  pixbuf loaders" argument from the Windows script doesn't cover. The smoke
  test is what keeps that claim honest, which is one more reason not to
  weaken it.
- macOS runners are 10× minutes on private repos; this repo is public, where
  they are free. If the repo ever goes private, the macos job is the first
  thing that gets expensive.
