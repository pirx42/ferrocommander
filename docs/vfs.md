# VFS — the virtual filesystem layer

← Parent: [CLAUDE.md](CLAUDE.md)

`tc-core::vfs` is the only way anything in this project touches a filesystem.
The UI never calls `std::fs`; it holds a `VirtualFs` and asks that.

## The interface

```rust
pub trait VirtualFs {
    fn read_dir(&self, path: &VfsPath) -> Result<Vec<Entry>, VfsError>;
    fn stat(&self, path: &VfsPath) -> Result<Entry, VfsError>;
}
```

Object-safe on purpose: a pane holds a `Box<dyn VirtualFs>` and swaps it when
the user steps into an archive, without knowing which backend answers.

**Read-only in phase 1.** The design doc lists `open/read/write, rename,
mkdir, remove` on this trait. They are absent until phase 2 brings the
operation engine that implements *and tests* them — trait methods added ahead
of a caller are dead code no test can pin down (skill
[20](skills/20-no-backward-compat-shims.md)).

## `VfsPath`

An always-absolute, always-normalized, `/`-separated path. `/` is used on
every platform because archive-internal paths are `/`-separated by
specification, and one separator keeps the type free of per-backend cases.

Normalization is **lexical**: `..` pops the previous component without
consulting the filesystem. Going up from `/link/sub` therefore returns to
`/link` — where the user believes they are — rather than to the symlink
target's parent. `..` cannot escape the root, and no input is invalid, so the
type has no error case and needs no `Result`.

## `Entry`

`name`, `kind`, `size`, `modified`, `hidden`.

- **`kind`** is `Dir`, `File`, or `Symlink(SymlinkTarget)` where the target is
  `Dir`, `File`, or `Broken`. Links keep their own variant rather than being
  resolved away, so the UI can render them as links while still knowing
  whether they are enterable — that question is `Entry::is_dir()`, true for
  directories *and* symlinks to directories.
- **`Broken`** is an ordinary outcome, not an error. A dangling link is a
  normal thing to find, and a file manager that refuses to list a directory
  because of one is useless.
- **`size`** is 0 for directories. A directory's inode size tells the user
  nothing and would turn size-sorting into noise; the UI renders `<DIR>`.
- **`hidden`** is decided by the *backend*, not by the listing layer. On Unix
  it is the leading dot; on Windows it is `FILE_ATTRIBUTE_HIDDEN`, which
  cannot be derived from the name at all. A dot-prefixed file such as
  `.gitignore` is an ordinary visible file on Windows, matching Total
  Commander. Filtering on the flag stays platform-agnostic.

`read_dir` returns entries **unsorted** — ordering is the listing layer's
decision — and never filters. Hidden entries are listed and flagged.

## Errors

`VfsError` is a closed set: `NotFound`, `PermissionDenied`, `NotADirectory`,
`Io(String)`. It deliberately carries no `io::Error`, so the variants stay
comparable for tests and the UI has a finite set of cases to render. The
original message survives in `Io` for the job log.

## Platform differences

All of them live in `vfs/platform.rs`, behind three functions — `to_std_path`,
`is_hidden`, `root_entries`. Adding a platform touches exactly one file.

| | Linux | Windows |
|---|---|---|
| Native path | the VFS path itself | `/C:/Users/pirx` → `C:\Users\pirx` |
| Hidden | leading dot in the name | `FILE_ATTRIBUTE_HIDDEN` |
| VFS root `/` | the real root directory | synthetic: the list of drives |

**Why the Windows root is the drive list.** Windows has no single filesystem
root, so `/` has to mean *something*. Making it the drive list keeps `VfsPath`
uniform across platforms and hands the design doc's drive selector to the UI
for free — a pane at `/` on Windows simply shows `C:`, `D:`, … as directories,
which is what Total Commander does.

## Testing

`crates/tc-core/tests/local_fs.rs` runs against real tempdirs and asserts
invariants over the fixture — name sets, entry counts, byte sums — rather than
hand-copied expected vectors, so a failure means the behavior changed (skill
[52](skills/52-test-conservation-invariants.md)).

Symlink tests are `#[cfg(unix)]`: creating a symlink on Windows requires
elevated privileges, so the tests would fail for reasons unrelated to the
code. `LocalFs` itself handles symlinks on both platforms.

Because half of `platform.rs` is invisible to a Linux build, the green gate
cross-checks the other half:

```bash
cargo clippy -p tc-core --all-targets --target x86_64-pc-windows-gnu -- -D warnings
```

This is not ceremony — it caught a Windows-only build break (an import used
only inside the `cfg(unix)` test module) on the very first run.

The check covers `tc-core` only. Cross-checking `tc-app` would need GTK's
`-sys` build scripts to find a mingw libgtk-4 through pkg-config, which a
Linux box does not have. That is an acceptable boundary because **every
platform-divergent line lives in `tc-core`** — `tc-app` contains no `cfg`
branches at all. Verifying the Windows GTK build needs a real Windows or
mingw toolchain.

## Known gap

If an entry disappears between `read_dir` enumerating it and `stat` reading
its metadata, the whole listing fails with `NotFound` instead of omitting the
vanished entry. Every alternative was untestable without an injection seam, so
the honest version shipped; the refresh logic in phase 3 is the right place to
revisit it.
