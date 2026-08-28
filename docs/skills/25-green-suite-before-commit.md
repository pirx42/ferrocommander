# Skill: Green Suite Before Every Commit

**When.** Before every `git commit`.

**Rule.** Format check, lint, the complete test suite, and the build run
**green** before a commit is made. If one of them is red:
**fix it, don't commit**.

**Why.** Red commits burn time for everyone who checks out afterwards.
Bisect breaks. CI credits are wasted. "I'll fix it right after" is in
practice often "I'll fix it in three weeks".

**How.**

Standard sequence (Rust workspace):
```bash
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo build --release
```
For pure documentation commits the suite can be skipped; for code changes
it is mandatory.

`cargo clippy` with `-D warnings` is the CI gate: ONE warning makes the
workflow fail — including in test code.

If a step is red:
- **fmt red:** run `cargo fmt --all`.
- **clippy red:** rule violation → restructure the code, don't disable the
  lint via `#[allow]` (unless with a documented reason).
- **test red:** see [24-no-silent-test-changes.md](24-no-silent-test-changes.md)
  for the decision (test correct? code correct?).
- **build red:** feature flags, missing dep, release-specific path.
  Not "works in debug anyway".

**Example routine.**
```bash
# After every code change:
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release

# Only then:
git add <files>
git commit -m "..."
```

**After a merge: no second run without changes.** The gate runs BEFORE
the merge, on the topic branch. If the target branch after the merge is
content-identical to the merged state — no file, neither code nor data,
differs — then the merge produced nothing newly-untested and the suite is
NOT run again (owner spec 2026-08-14). A second run there proves nothing,
costs minutes, and only produces new opportunities for flakes.

The check is a tree comparison, not a gut feeling:
```bash
git diff --quiet topic/<plan> dev && echo "identical → no re-run"
```
If the diff is NOT empty, the merged state is different from the tested
one (foreign commits on dev in between, conflict resolution, touch-ups in
the merge commit) — then the full gate runs on the merge result.

**Anti-patterns.**
- `git commit` without `cargo test`.
- **Pipe exit trap:** `cargo test | tail -3` (or similar) masks the exit
  code — the pipe reports the status of `tail`, so an `&&` gate continues
  despite red tests (actually happened 2026-07-05: 3 failures went
  unnoticed). Either set `set -o pipefail` or explicitly check the
  summary line for `failed`.
- "only ran tsc, forgot the build" — the build covers path aliases and
  production-specific paths.
- Leaving it red with a note "flaky" — usually it's not flaky, it's
  genuinely broken.

**Related.**
- [33-never-skip-hooks.md](33-never-skip-hooks.md) (pre-commit hooks
  catch this automatically).
- [23-tests-accompany-commits.md](23-tests-accompany-commits.md)
