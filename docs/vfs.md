# VFS — the virtual filesystem layer

← Parent: [CLAUDE.md](CLAUDE.md)

`tc-core::vfs` is the only way anything in this project touches a filesystem.
The UI never calls `std::fs`; it holds a `VirtualFs` and asks that.

## The interface

```rust
pub trait VirtualFs: Send + Sync {
    // reading
    fn read_dir(&self, path: &VfsPath) -> Result<Vec<Entry>, VfsError>;
    fn stat(&self, path: &VfsPath) -> Result<Entry, VfsError>;
    // writing
    fn create_dir(&self, path: &VfsPath) -> Result<(), VfsError>;
    fn remove_dir(&self, path: &VfsPath) -> Result<(), VfsError>;
    fn remove_file(&self, path: &VfsPath) -> Result<(), VfsError>;
    fn rename(&self, from: &VfsPath, to: &VfsPath) -> Result<(), VfsError>;
    fn open_read(&self, path: &VfsPath) -> Result<Box<dyn Read + Send>, VfsError>;
    fn create_file(&self, path: &VfsPath) -> Result<Box<dyn Write + Send>, VfsError>;
    fn set_modified(&self, path: &VfsPath, time: SystemTime) -> Result<(), VfsError>;
    fn trash(&self, path: &VfsPath) -> Result<(), VfsError>;
}
```

Object-safe on purpose: a pane holds a `dyn VirtualFs` and swaps it when the
user steps into an archive, without knowing which backend answers.

## Why the write side looks like this

**`Send + Sync` is a bound, not a feature.** A file operation runs on a worker
thread and holds its source and target backends across it. Declaring it costs
nothing today — `LocalFs` is a unit struct — and it tells phase 6's
`ArchiveFs`, which owns an open archive handle, that it needs interior
mutability *before* it is half written rather than after.

**`remove_dir` refuses a non-empty directory.** Recursion is the operation
engine's walk. Only the engine can report per-file progress, log a per-file
error and carry on, and honour a cancel between two entries; a recursive
backend delete would be a second, silent implementation of the same walk with
none of that. `create_dir` is non-recursive for the same reason.

**`rename` reports `CrossDevice` instead of falling back to a copy.** The
copy+delete degradation is a decision with a progress bar and a rollback
attached, which makes it engine policy, not backend behavior.

**Streams, not a `copy_file` method.** One `Read`/`Write` pair means the copy
loop exists once and works local→local today and local→archive in phase 6
without a second code path.

**`create_file` truncates an existing file.** Whether overwriting is allowed
is decided before the call — the engine has to ask the user anyway, and a
second existence check in the backend would be a second answer to the same
question.

**`trash` sits on the trait rather than in the engine**, because only a
backend knows whether its storage has such a thing. Phase 6's archives will
not.

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

**An entry that disappears mid-listing is dropped, not fatal.** A file
deleted between the directory being enumerated and its metadata being read is
simply left out — failing the whole listing over one file that a job elsewhere
removed half a millisecond ago would be worse than a listing that is one row
short. Anything other than "not found" still fails the listing, because
silently returning a short directory would be a lie about what is there. That
rule is a named function with its own test; provoking the race itself would
need a seam the standard library does not offer.

`read_dir` stats every entry relative to the directory it already has open
(`DirEntry::metadata`) rather than re-resolving each full path, which is the
same answer for a third less work — see [performance.md](performance.md). A
path is built only for the entries that turn out to be symlinks, whose target
needs a second look.

`read_dir` returns entries **unsorted** — ordering is the listing layer's
decision — and never filters. Hidden entries are listed and flagged.

## Errors

`VfsError` is a closed set. It deliberately carries no `io::Error`, so the
variants stay comparable for tests and the UI has a finite set of cases to
render. The original message survives in `Io` for the job log.

| Variant | Raised by |
|---|---|
| `NotFound` | anything aimed at a path that is not there |
| `PermissionDenied` | the filesystem refusing the caller |
| `NotADirectory` | listing something that is not a directory |
| `AlreadyExists` | `create_dir` onto an existing name — the engine turns this into a conflict prompt, not an error |
| `NotEmpty` | `remove_dir` on a directory with contents |
| `IsADirectory` | `remove_file` aimed at a directory |
| `CrossDevice` | `rename` across filesystems; the engine answers with copy + delete |
| `Io(String)` | anything unmodelled, description preserved |

## Platform differences

All of them live in `vfs/platform.rs`. Adding a platform touches exactly one
file.

| | Linux | Windows |
|---|---|---|
| Native path | the VFS path itself | `/C:/Users/pirx` → `C:\Users\pirx` |
| Hidden | leading dot in the name | `FILE_ATTRIBUTE_HIDDEN` |
| VFS root `/` | the real root directory | synthetic: the list of drives |
| Trash errors | the crate wraps the real `io::Error`, so `NotFound` survives | Win32 status codes, kept as `Io` |

**Why trash errors are a platform function.** The `trash` crate's error
*shape* differs by target: its freedesktop backend carries the underlying
`io::Error`, its Windows backend reports Win32 codes. Unwrapping the former
keeps a failed trash inside the same closed error set as every other call —
trashing a path that is already gone reports `NotFound` rather than an opaque
string. Hand-mapping Win32 codes would be a table of guesses, so Windows keeps
the description in `Io`.

The crate's `Error::source()` is not a way around the split: for its
filesystem variant it returns the io error's *own* source, which is `None`,
not the io error itself.

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

## Known gaps

**`set_modified` cannot stamp a directory.** Stamping needs a handle opened
for writing, which no platform hands out for a directory, so a copied
directory carries the time it was created rather than the original's. Files —
which is what the size and date columns are about — keep their date.

**Copies do not carry permission bits.** `Entry` has no mode, so an
executable script copied through the engine arrives without its `+x`. A
portable permission model belongs with phase 3's attributes column.

Both live in [future-improvements.md](future-improvements.md) with their
reasons.
