# VFS — the virtual filesystem layer

← Parent: [CLAUDE.md](CLAUDE.md)

`fc-core::vfs` is the only way anything in this project touches a filesystem.
The UI never calls `std::fs`; it holds a `VirtualFs` and asks that.

## The interface

```rust
pub trait VirtualFs: Send + Sync {
    // what this backend is
    fn store(&self) -> Store;
    fn read_only(&self) -> bool { false }
    fn space(&self, _path: &VfsPath) -> Option<Space> { None }
    // reading
    fn read_dir(&self, path: &VfsPath) -> Result<Vec<Entry>, VfsError>;
    fn stat(&self, path: &VfsPath) -> Result<Entry, VfsError>;
    fn open_read(&self, path: &VfsPath) -> Result<Box<dyn Read + Send>, VfsError>;
    fn read_at(&self, path: &VfsPath, offset: u64, len: usize) -> Result<Vec<u8>, VfsError>;
    // writing
    fn create_dir(&self, path: &VfsPath) -> Result<(), VfsError>;
    fn remove_dir(&self, path: &VfsPath) -> Result<(), VfsError>;
    fn remove_file(&self, path: &VfsPath) -> Result<(), VfsError>;
    fn rename(&self, from: &VfsPath, to: &VfsPath) -> Result<(), VfsError>;
    fn create_file(&self, path: &VfsPath) -> Result<Box<dyn Write + Send>, VfsError>;
    fn set_modified(&self, path: &VfsPath, time: SystemTime) -> Result<(), VfsError>;
    fn set_attributes(&self, path: &VfsPath, attributes: Attributes) -> Result<(), VfsError>;
    fn trash(&self, path: &VfsPath) -> Result<(), VfsError>;
}
```

`read_at` is random access rather than a seekable reader, because a seekable
reader is a promise not every backend can keep: an entry inside a compressed
[archive](archives.md) has no cheap seek. It is what the [viewer](viewer.md)
is built on.

`space` is the free-and-total figure under each pane's status line
([ui-shell.md](ui-shell.md)). It is asked of the **backend** rather than of a
path, and defaults to `None`: an archive has no free space of its own, and
reporting the disk the archive file happens to sit on would answer a question
nobody asked.

Object-safe on purpose: a pane holds a `dyn VirtualFs` and swaps it when the
user steps into an archive, without knowing which backend answers.

## Why the write side looks like this

**`Send + Sync` is a bound, not a feature.** A file operation runs on a worker
thread and holds its source and target backends across it. It costs `LocalFs`,
a unit struct, nothing, and it is what shaped `ArchiveFs`: a reader that
borrowed a shared archive could never be `Send`, so every reader owns its
range of the container instead ([archives.md](archives.md)).

**`remove_dir` refuses a non-empty directory.** Recursion is the operation
engine's walk. Only the engine can report per-file progress, log a per-file
error and carry on, and honour a cancel between two entries; a recursive
backend delete would be a second, silent implementation of the same walk with
none of that. `create_dir` is non-recursive for the same reason.

**`rename` reports `CrossDevice` instead of falling back to a copy.** The
copy+delete degradation is a decision with a progress bar and a rollback
attached, which makes it engine policy, not backend behavior.

**Streams, not a `copy_file` method.** One `Read`/`Write` pair means the copy
loop exists once. It is what makes unpacking an archive the ordinary copy
engine reading one backend and writing another, with no second code path and
no idea that it is unpacking ([archives.md](archives.md)).

**`create_file` truncates an existing file.** Whether overwriting is allowed
is decided before the call — the engine has to ask the user anyway, and a
second existence check in the backend would be a second answer to the same
question.

**`trash` sits on the trait rather than in the engine**, because only a
backend knows whether its storage has such a thing. [Archives](archives.md)
do not: `trash` reports `ReadOnly` there like every other mutating call, which
is what the trait had already predicted an archive would need.

## `VfsPath`

An always-absolute, always-normalized, `/`-separated path. `/` is used on
every platform because archive-internal paths are `/`-separated by
specification, and one separator keeps the type free of per-backend cases.

Normalization is **lexical**: `..` pops the previous component without
consulting the filesystem. Going up from `/link/sub` therefore returns to
`/link` — where the user believes they are — rather than to the symlink
target's parent. `..` cannot escape the root, and no input is invalid, so the
type has no error case and needs no `Result`.

## `Store` — which storage a backend addresses

Every backend reports one. Two backends reporting the same `Store` speak the
same paths: a `rename` from one to the other means something, and comparing a
path in one with a path in the other is a real comparison. Two reporting
different stores share nothing but the shape of a path.

`LocalFs` is one store, always. Each opened archive is a store of its own.

This is not bookkeeping. A move's fast path is a single `rename`, and running
it across two stores hands the source's path to the target backend — where
`/packed.txt` from inside an archive names a file at the root of the disk. See
[archives.md](archives.md) and [ops.md](ops.md).

## `read_only` — a backend that cannot be written to at all

A property of the backend, not of a path, so it is answered once instead of
discovered per file. The operation engine asks before it scans: a copy of a
large tree into an archive is one refusal with a reason rather than a thousand
identical ones. A **delete** asks the source backend, because that is the one
it removes from.

Defaulted to `false`, because a backend that can be written to has nothing to
say here and the one that cannot is the exception.

## `Entry`

`name`, `kind`, `size`, `modified`, `attributes`, `hidden`.

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
- **`attributes`** is an opaque `Copy` value, not a rendered string: the pane
  renders it for the Attr column and the copy engine restores it onto the
  copy, and both need the same thing. Only `vfs::platform` reads the bits,
  because what they mean is entirely different on the two platforms. A copy
  restores them *after* the timestamp — taking write permission away first
  would stop the timestamp being set at all.
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
| `ReadOnly` | any mutating call on a backend that has no write side at all, such as an [archive](archives.md) |
| `NotAnArchive` | opening a file as an archive when it is not one this build reads |
| `Io(String)` | anything unmodelled, description preserved |

## Platform differences

All of them live in `vfs/platform.rs`. Adding a platform touches exactly one
file **here**; what it costs in `fc-app` is a different question, and
[future-improvements.md](future-improvements.md) prices it against macOS —
where the engine is nearly free because macOS *is* `unix`, and the keymap, the
packaging and the end-to-end suite are not.

| | Linux | macOS | Windows |
|---|---|---|---|
| Native path | the VFS path itself | the VFS path itself | `/C:/Users/pirx` → `C:\Users\pirx` |
| Hidden | leading dot in the name | leading dot, **or** `UF_HIDDEN` — Finder's flag on `~/Library` | `FILE_ATTRIBUTE_HIDDEN` |
| VFS root `/` | the real root directory | the real root directory | synthetic: the list of drives |
| Attributes | Unix mode bits, shown as `rwxr-xr-x` | the same — macOS *is* `unix` here | Win32 file attributes, shown as `RHSA` |
| Setting them | the full permission bits | the full permission bits | the read-only flag only — `std` sets nothing else |
| Mount points | `/proc/self/mounts`, minus the kernel's own | `getfsstat(2)`, minus `devfs`, `autofs` and `/System/Volumes` — the **same judge**, macOS lists | the drive list |
| Settings live in | `$XDG_CONFIG_HOME`, else `~/.config` | `$XDG_CONFIG_HOME` when set, else `~/Library/Application Support` | `%APPDATA%` |
| Trash errors | the crate wraps the real `io::Error`, so `NotFound` survives | kept as `Io` — the crate's macOS backend carries no `io::Error` to unwrap | Win32 status codes, kept as `Io` |

**The macOS column is compile-checked and fixture-tested, never yet run.**
Its `mount_points` lists and the `getfsstat` reader are asserted from
documentation; the judgement over them is the same shared function the Linux
fixtures exercise, and the macOS lists have fixture tests of their own that
run here. What a real Mac must review first is written into
`platform.rs` and the
[groundwork plan](plans/archive/2026-08-30-macos-groundwork.md) § 5.

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

`crates/fc-core/tests/local_fs.rs` runs against real tempdirs and asserts
invariants over the fixture — name sets, entry counts, byte sums — rather than
hand-copied expected vectors, so a failure means the behavior changed (skill
[52](skills/52-test-conservation-invariants.md)).

Symlink tests are `#[cfg(unix)]`: creating a symlink on Windows requires
elevated privileges, so the tests would fail for reasons unrelated to the
code. `LocalFs` itself handles symlinks on both platforms.

Because parts of `platform.rs` are invisible to a Linux build, the green
gate cross-checks the other targets:

```bash
cargo clippy -p fc-core --all-targets --target x86_64-pc-windows-gnu -- -D warnings
cargo clippy -p fc-core --all-targets --target aarch64-apple-darwin -- -D warnings
```

The Windows one is not ceremony — it caught a Windows-only build break (an
import used only inside the `cfg(unix)` test module) on the very first run.
The macOS one was green before any macOS branch existed — 7.6 s warm, every
dependency compiling — which is worth knowing precisely: it means a compile
check cannot see the Linux assumptions living inside `cfg(unix)`, and the
macOS branches this gate watches had to be *written* before it watched
anything ([the groundwork plan](plans/archive/2026-08-30-macos-groundwork.md)
is the record).

The check covers `fc-core` only. Cross-checking `fc-app` would need GTK's
`-sys` build scripts to find a target libgtk-4 through pkg-config, which a
Linux box does not have. That is an acceptable boundary because **every
platform-divergent *behaviour* lives in `fc-core`** — with one counted
exception: the three `cfg` markers in `keymap.rs` that ship the dormant macOS
key layer, which cannot live in the engine because an `Action` is a shell
concept the engine deliberately does not know ([config.md](config.md)). Those
three lines are therefore the only platform code no gate cross-checks. What
keeps that honest is that the *layer itself* is data, applied and asserted
unconditionally by tests on Linux — only the two-line wiring in
`Keymap::default` is beyond every check, and it is named here rather than
rounded away. (This sentence used to say `fc-app` contains no `cfg` at all,
and was true until the layer landed.) The Windows GTK build itself was
verified by hand on
2026-08-30 — MSYS2 MINGW64 with GTK4 4.22.4 — and it compiles and runs; what
it needs in order to start is in [ui-shell.md](ui-shell.md). That was a
one-off on a developer machine, not something the gate can do.

## Known gaps

**`set_modified` cannot stamp a directory.** Stamping needs a handle opened
for writing, which no platform hands out for a directory, so a copied
directory carries the time it was created rather than the original's. Files —
which is what the size and date columns are about — keep their date. Packing
an archive is the exception that can: it writes the date into the entry rather
than onto the filesystem, so it reads the source directory's date with one
extra `stat` ([archives.md](archives.md)).

It lives in [future-improvements.md](future-improvements.md) with its reason.

## Which mounts get a drive button

`parse_mount_table` decides, and the bar for inclusion is "somewhere a person
navigates to". Two filters: a list of pseudo **filesystem types**, and a list
of pseudo **roots** — `/proc`, `/sys`, `/dev`, `/run` — matched as "this path
or a path inside it".

**That second list used to carry trailing slashes**, so it excluded the
children of `/run` and not `/run` itself. `/run` is a `tmpfs`, and `tmpfs` is
not a pseudo type, so it passed both filters and got a button. On the machine
where this was found it took the *first* one, ahead of `/` — which is what
five tests indexing the drive list positionally then walked into.

**`tmpfs` is deliberately not a pseudo type**, which would have been the other
way to exclude `/run` and is the wrong one: `/tmp` is a `tmpfs` on plenty of
machines and is somewhere people go daily. A test pins that.

**`squashfs` is one**, since this: every snap on an Ubuntu desktop is a
read-only squashfs image, a stock machine has twenty-six of them, and left in
they push the actual disks off the end of a bar that is supposed to be a
shortcut. A squashfs somebody loop-mounted to look inside is still reachable
by typing its path.

## Which drive a path is on

`mount_for(path, mounts)` answers it, and answers with the **longest** mount
the path is at or inside: mounts nest, so a file under `/mnt/backup` belongs to
the backup drive rather than to `/`. Taking the first match instead would put
everything on `/` on Unix, where `/` is a prefix of every path there is. The
mounts are a parameter rather than read inside, so the rule is testable against
a made-up machine.

`VfsPath::is_inside` is what it compares with, and it compares **whole
components**: `/home/pirx2` is not inside `/home/pirx`. Reading it as a text
prefix is how a copy comes to refuse a perfectly good target — or, worse, to
accept a job that writes into its own source, which is why `ops` uses the same
function.

That component walk also settles the root, which a prefix test got wrong in
the dangerous direction: stripping `/` off `/home` leaves `home`, which starts
with no separator, so `/home` read as *not* inside `/`. Nothing noticed while
only `ops` used it; the mount rule asks that question about every path there
is, and failed on the first one.
