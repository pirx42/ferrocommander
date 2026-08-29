# An Ubuntu package, built on every commit to `main`

**Status:** Draft — awaiting approval, nothing implemented
**Wants its own topic branch**, per skill
[10](../skills/10-plan-lifecycle.md).

A `.deb` somebody can download and install, rebuilt whenever `main` moves.

This is the first thing this repository has ever built for anybody other than
the person who checked it out, and it is worth saying what that changes: the
audience stops being "a developer with `cargo`" and becomes "somebody with
Ubuntu". Everything below follows from that.

## 1. What was decided, and by whom

Four questions were put to the owner; all four took the recommendation.

| Question | Answer |
|---|---|
| Where does the `.deb` go? | **A rolling GitHub pre-release**, replaced on every commit — a stable URL to `wget`, no extra infrastructure, no accumulating builds. |
| Does the workflow verify the commit first? | **Yes, the full green gate**, and it packages only if green. |
| What does the package install? | **The binary, a `.desktop` entry and an icon** — the three things that make it an application rather than a file in `/usr/bin`. |
| How is it versioned, and for which Ubuntu? | **`0.1.0-<commit count>`, built on 24.04.** |

Six more decisions had no reason to bother the owner with:

- **The logic goes in `scripts/package-deb.sh`; the workflow calls it.**
  This is the load-bearing decision of the whole plan. A GitHub workflow can
  only be tested by pushing to GitHub — there is no local run, no probe, no
  green gate. So the workflow must contain as close to nothing as possible:
  check out, install the build dependencies, run two scripts, upload. Every
  line of actual logic lives in a script that can be run, read and fixed on a
  laptop. `scripts/green-gate.sh` already sets that precedent, and for
  exactly the same reason — a chain typed at a prompt is a chain nobody can
  check.
- **`cargo-deb` rather than a hand-written `control` file.** The runtime
  dependencies of a GTK binary are a long list nobody should maintain by
  hand, and `cargo-deb` derives them from the ELF via `dpkg-shlibdeps` — the
  same way Debian's own tooling does. It reads its configuration from
  `[package.metadata.deb]`, so the packaging description lives in the
  manifest beside the crate it describes.
- **First-party actions and the `gh` CLI only.** `actions/checkout` and the
  `gh` already on the runner. A third-party action for the release upload
  would be one more thing to pin by SHA and audit; `gh release` is two lines.
- **The workflow gets `contents: write` and nothing else.** It has to replace
  a release. Every other permission is explicitly `read`, because a workflow
  that runs on every commit to `main` is the most attractive thing in the
  repository to whoever compromises a dependency.
- **The `.desktop` file and the icon are repository files**, under
  `packaging/`, not strings inside a script. They are content a person edits
  and looks at, and `st.rose.Ferrocommander` — the application id the shell
  already registers — is what names both.
- **A failed gate fails the workflow.** No "package anyway and mark it
  broken": the point of gating is that the artefact means something.

## 2. What this costs the existing code

Checked against the repository rather than guessed
(skill [65](../skills/65-verify-or-ask-never-assume.md)):

| | State |
|---|---|
| Any CI at all | **None.** There is no `.github/` directory. This is the first workflow, and `CLAUDE.md` already calls clippy "the lint gate (CI)" — a claim nothing has been making true. |
| A verifiable build sequence | **Exists**, `scripts/green-gate.sh`, six steps, already the rule before every commit. |
| A build number | **Exists.** `crates/tc-app/build.rs` computes `git rev-list --count HEAD` and stamps it into the window title. The package version reuses **that** number, so a title bar and a `dpkg -l` line agree about which build somebody is running. |
| An application id | **Exists**, `st.rose.Ferrocommander`, in `constants.rs`. |
| An icon | **Does not exist.** Nothing in the repository is an image. One has to be drawn. |
| A `.desktop` file | Does not exist. |
| GTK ≥ 4.12 | The reason the target is **24.04**: it ships GTK 4.14, and 22.04 ships 4.6. The manifest already records this trade in the `gtk` dependency comment. |

**Three things are wrong today and are found by trying to package them:**

1. **The binary is called `tc-app`.** That is the crate's name, and a fine
   name for a crate; as `/usr/bin/tc-app` it is a name nobody typed on
   purpose. It becomes `ferrocommander`, which costs a `[[bin]]` section and
   a change in the end-to-end harness, where `CARGO_BIN_EXE_tc-app` names it.
2. **`repository` in the workspace manifest is wrong** —
   `github.com/pirx/ferrocommander`, where the remote is `pirx42/`. It would
   have gone straight into the package's `Homepage` field.
3. **There is no description fit for a package.** `tc-app`'s is "GTK4 shell:
   dual-pane window, dialogs, viewer", which describes a crate to a
   developer. `apt show` wants a sentence about what the program is for.

## 3. The awkward corners, each of which gets a check

Reliability here does not mean tests in the usual sense — a workflow has no
unit tests. It means **the parts that can be run locally are run locally**,
and the parts that cannot are as small as possible.

- **The package installs and the binary starts.** `scripts/package-deb.sh`
  can be run in this container; the result can be installed with `dpkg -i`
  and the binary invoked. That is the check, and it is a real one.
- **The declared dependencies are the actual ones.** `dpkg-shlibdeps` is
  trusted to find them; what is checked is that the resulting `Depends:` is
  not empty and names a GTK 4 runtime — a package that installed cleanly on a
  machine with no GTK and then failed to start is the failure mode.
- **The `.desktop` file is valid**, by `desktop-file-validate`, which Ubuntu
  ships. An invalid one does not stop installation; it stops the launcher
  entry appearing, silently.
- **The version is what the title bar says.** The one number, from one
  source, checked by reading both.
- **An uninstall leaves nothing behind**, which `dpkg -r` and a `find` will
  say.
- **A second run replaces the release rather than adding to it.** The only
  claim in this plan that cannot be checked before it runs for real; it is
  two lines of `gh`, and it is called out here as the part to watch on the
  first push.

## 4. Phases — one phase, one commit

Docs ride in the commit that changes the behaviour
(skill [28](../skills/28-docs-in-same-commit.md)).

### Phase 0 — the three things packaging finds

The binary rename, the wrong repository URL, the package description. All
three are in § 2, all three are prerequisites rather than packaging, and each
is a change to the crate rather than to any workflow — so they go first,
where the existing suite can catch what they break. The end-to-end harness
names the binary and has to be updated with them.

No probes here: this is not code with behaviour to pin, it is metadata. What
*is* checked is that the suite still passes with the binary renamed, which is
the one thing the rename can break.

### Phase 1 — the package, built by a script

`packaging/st.rose.Ferrocommander.desktop`, an icon, `[package.metadata.deb]`
in `crates/tc-app/Cargo.toml`, and `scripts/package-deb.sh` that produces the
`.deb`. Run here, installed here, started here.

**The icon is drawn rather than borrowed** — a two-pane mark, as an SVG, at
`packaging/`. It will not be good art. It will be honest, ours, and
replaceable, which is what matters for a first one.

### Phase 2 — the workflow

`.github/workflows/main.yml`: on push to `main`, on Ubuntu 24.04 — install
the build dependencies, run `scripts/green-gate.sh`, run
`scripts/package-deb.sh`, replace the rolling pre-release with `gh`.

The workflow file is the part that cannot be tested before it runs. It is
therefore the *shortest* file in this plan, and every line in it that is not
a call to something already tested is a line to argue about.

### Phase 3 — the documentation nobody has needed until now

A `README.md` — the repository has none, which was defensible while the
audience was one person with `cargo` and is not once there is a package to
install. What it is, what it needs, where the `.deb` is, how to build from
source. Plus `docs/packaging.md` for how the package is made, and the
`CLAUDE.md` cookbook row.

### Phase 4 — refactoring audit

Skill [49](../skills/49-final-phase-refactoring-audit.md), and the one
question this plan cannot answer from a laptop: **watch the first real run**,
and fix what it says. A plan that ends before its workflow has ever run has
not finished.

## 5. What is deliberately not in this

- **An apt repository.** Named in the question and declined for now: a
  signing key held as a secret, an index rebuilt per commit, and a Pages
  deployment, for an audience that does not exist yet. The release URL is
  enough to find out whether anybody wants it.
- **Windows and macOS builds.** Both are supported *targets*; neither is a
  package, and the cross-build story for each is its own plan.
- **Tagged version releases.** This builds `main`, always as a pre-release.
  Real versioned releases need a version somebody bumps on purpose, which is
  a decision this repository has not had to make yet.
- **Signing the `.deb`.** Meaningful inside an apt repository, mostly
  decorative on a file downloaded over HTTPS from a URL you already trust.

## 6. Effort

Factor 0.25 per skill [45](../skills/45-calibrate-effort-estimates.md).

| Phase | Raw | Corrected |
|---|---|---|
| 0 — the three prerequisites | ~2 h | ~30 min |
| 1 — the package and its script | ~5 h | ~1.25 h |
| 2 — the workflow | ~2 h | ~30 min |
| 3 — README and packaging doc | ~3 h | ~45 min |
| 4 — audit, and the first real run | ~2 h | ~30 min |
| **Total** | | **~3.5 h** |

**The factor is least trustworthy here**, and it is worth saying so rather
than presenting a number with the same confidence as the last four plans. All
of those were Rust in a repository with a green gate and a probe for every
claim. This one ends in a system that cannot be run before it is pushed —
`cargo-deb`'s behaviour on this particular binary, the runner's package list,
and the release upload are each a thing that can be wrong in a way no local
check will reveal. Phase 2 is thirty minutes of writing and possibly an hour
of watching runs fail for reasons a laptop cannot reproduce.
