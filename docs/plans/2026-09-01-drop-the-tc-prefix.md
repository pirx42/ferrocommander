# `tc` means Total Commander, and nothing else

Status: Draft — §2's two decisions are open

The crates are called `tc-core` and `tc-app`, from a working title the
project outgrew: it is *FerroCommander*, and `tc` in a crate name says
"Total Commander" about code that is not Total Commander. The owner's rule
is one line: **`tc` is allowed only when it refers to Total Commander
itself** — the program this one imitates, named as `TC` in prose and
`tc-clone` in the design plan's filename. Everywhere else it goes.

## 1. What is actually there

441 occurrences across 98 files, in three zones that want different
treatment:

| Zone | Count | What it is |
|---|---|---|
| code, manifests, scripts, workflow | 245 | `crates/tc-core/`, `crates/tc-app/`, `use tc_core::…`, `-p tc-core`, `--package tc-app`, the workspace members and the path dependency |
| live docs | 69 | every `CLAUDE.md`, the README's layout table and build line, the topic docs that name a module's home |
| archived plans | 127 | historical records that named the crates while describing work already done (§2, decision 2) |

And the **legitimate** `tc`, which stays by the owner's own rule: `TC's own
quirk` and the like in [keymap.md](../keymap.md), the root CLAUDE.md's *TC
Clone Linux* title, and `2026-08-28-tc-clone-design.md` — each of those is
Total Commander being named.

Three details the rename has to get right rather than sed over:

- **The binary is already `ferrocommander`** and the `.deb` package is
  already `ferrocommander`. Neither changes; what changes is only the crate
  around them. The `[[bin]]` comment in the app manifest argues its case in
  terms of `tc-app` and needs rewriting rather than substituting — the
  argument survives, its example does not.
- **`Cargo.lock` carries the names** and regenerates itself on the first
  build; it is committed, so the rename lands there too.
- **Nothing today fails when the old name reappears.** The compiler catches
  a `use fc_core` that should be `use ferro_core`, but nothing catches a
  *new* `tc-` creeping into a comment or a doc. That gap is phase 3.

## 2. Decisions before implementation

1. **What the crates are called instead.** Recommended: **`fc-core` /
   `fc-app`** — the exact structural twin of what is there, so every
   `-p` flag, path and `use` line keeps its shape and the diff stays
   mechanical; `fc` is FerroCommander the way `tc` was Total Commander.
   Alternatives: `ferro-core` / `ferro-app`, more pronounceable and less
   cryptic at the cost of four characters in every `use ferro_core::…`;
   or `ferrocommander-core` / `ferrocommander-app`, unambiguous and long
   enough to be felt on every one of the 47 code files that import it.
2. **Whether the 127 archived-plan references change too.** Recommended:
   **yes, rewrite them.** A plan's reasoning is unchanged by what the
   thing is called, and a reader who greps `tc-core` should find nothing
   rather than 127 hits pointing at directories that no longer exist. The
   record stays true — only the name of a still-existing thing moves.
   Alternative: leave the archive frozen as a record of what things were
   called at the time, and accept that the repository keeps answering to
   the old name.

## 3. Phases

**Phase 0 — coverage pre-check** (skill 43). What guards this today: the
compiler (every `use`, every `-p`, every workspace member), the full green
gate, and `check-links.py` for the doc links that name crate paths. What is
*not* guarded is the property the owner actually asked for — that the
spelling is gone — which phase 3 adds rather than assumes.

**Phase 1 — the rename, in one commit.** `git mv` both crate directories;
the workspace members and the path dependency; both manifests, including
the reworded `[[bin]]` comment; every `use` and every `-p`/`--package`
flag in scripts and the workflow; `Cargo.lock`. And the live docs in the
same commit (skill 28), because a tree whose documents describe a layout
that no longer exists is worse than either half alone: the four
`CLAUDE.md` files, the README's layout table and its `cargo run -p` line,
and every topic doc that names a module's home.

**Phase 2 — the archived plans** (decision 2), separately, so the
mechanical sweep over 127 historical references is one revertible commit
rather than noise inside the real rename.

**Phase 3 — the guard.** A gate step that fails when the forbidden
spelling reappears anywhere outside its allowed use, so this cannot creep
back the way `tc-app` crept into a comment written after the crate was
named. The check is a script beside `check-links.py` — no toolchain, a
second to run — and the allowed uses are a short, named list rather than
an exception nobody can see: `TC` as a word in prose, and the
`tc-clone-design` filename. The gate gains it; the same script says what
it allows and why.

**Phase 4 — refactoring audit** (skill 49) and the end-of-plan ritual.
Read the whole change as one diff: what the rename *revealed* — comments
whose argument was about the old name, a doc sentence that only made sense
with `tc` in it — rather than what it merely substituted.

## 4. Effort

Corrected per skill 45; the last plans held at ~0.10.

| Phase | Raw | Corrected |
|---|---|---|
| 0 pre-check | 0.25 d | 0.25 h |
| 1 rename | 1 d | 1 h |
| 2 archive | 0.25 d | 0.25 h |
| 3 guard | 0.5 d | 0.5 h |
| 4 audit | 0.5 d | 0.5 h |

About two and a half hours of work; the wall clock is owned by three or
four full-gate runs at ~15 minutes each.

## 5. What would make this wrong

- **A sed is not a rename.** `tc_core` inside a word, a comment whose
  sentence is *about* the old name, and the legitimate `TC` are three
  different things a blind substitution treats alike. The guard in phase 3
  is what turns "I think I got them all" into something checkable, and the
  audit in phase 4 is what reads the sentences the substitution touched.
- **The archive decision is one-way in practice.** Rewriting 127
  historical references is easy; deciding later that the record should
  have been frozen means reading them all again.
- **Anything outside this repository that names the crates** — a checkout
  path in someone's notes, a `cargo run -p tc-app` in muscle memory — is
  not reachable from here. The README's build line is the one place a
  newcomer would have read it, and it moves in phase 1.
