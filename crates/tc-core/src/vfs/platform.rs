//! Everything about the local filesystem that differs between Linux, macOS
//! and Windows, collected in one module.
//!
//! macOS is `unix`, so it shares this module's `unix` half; where it differs
//! from Linux the split is `target_os` *inside* that half, and every such
//! branch is compile-checked by the gate's `aarch64-apple-darwin` step. What
//! no check on a Linux box can do is *run* them — the macOS-only lists and
//! the `getfsstat` reader are asserted from documentation, and
//! [`docs/plans/2026-08-30-macos-groundwork.md`] § 5 names them as the first
//! things a real Mac must review.
//!
//! Keeping the `cfg` split here rather than sprinkling it through `local.rs`
//! means the platform contract is a short, readable list — three functions —
//! and adding a platform touches exactly one file.
//!
//! The contract:
//! - [`to_std_path`] maps a [`VfsPath`] onto a native path.
//! - [`is_hidden`] decides whether an entry is hidden, per platform
//!   convention. This cannot be left to the listing layer: on Windows
//!   "hidden" is a file attribute that only the filesystem knows, not
//!   something derivable from the name.
//! - [`root_entries`] answers what the VFS root contains when the platform
//!   has no single filesystem root.
//! - [`from_std_path`] is the inverse of [`to_std_path`].
//! - [`home_dir`] locates the user's home directory, [`config_dir`] the
//!   place per-user settings belong.
//! - [`attributes`] and [`render_attributes`] read and show the permission
//!   or attribute bits, which are entirely different things on the two
//!   platforms; [`attributes_from_unix_mode`] takes the mode an archive
//!   recorded and says what it amounts to here.
//! - [`set_attributes`] puts them back onto a copy.
//! - [`mount_points`] lists the places the drive bar offers.
//! - [`trash_error`] maps a `trash` failure onto [`VfsError`]. It lives here
//!   because the crate's error *shape* differs by target: the freedesktop
//!   backend wraps the underlying `io::Error`, the Windows one does not.

use std::path::{Path, PathBuf};

use super::path::VfsPath;
use super::types::{Attributes, Entry, Mount, Space, VfsError};

#[cfg(unix)]
mod imp {
    use super::*;

    /// On Unix the VFS path *is* the native path — both are `/`-rooted.
    pub fn to_std_path(path: &VfsPath) -> PathBuf {
        PathBuf::from(path.as_str())
    }

    /// Unix permissions, as `st_mode`.
    pub fn attributes(metadata: &std::fs::Metadata) -> Attributes {
        use std::os::unix::fs::PermissionsExt;
        Attributes::from_raw(metadata.permissions().mode())
    }

    /// A mode an archive recorded is already what this platform means by
    /// attributes, so it is kept as it is.
    pub fn attributes_from_unix_mode(mode: u32) -> Attributes {
        Attributes::from_raw(mode)
    }

    /// And back out again, for writing one into an archive.
    pub fn unix_mode_of(attributes: Attributes) -> Option<u32> {
        Some(attributes.raw())
    }

    /// `rwxr-xr-x`, the form every Unix tool prints.
    pub fn render_attributes(attributes: Attributes) -> String {
        const FLAGS: [(u32, char); 9] = [
            (0o400, 'r'),
            (0o200, 'w'),
            (0o100, 'x'),
            (0o040, 'r'),
            (0o020, 'w'),
            (0o010, 'x'),
            (0o004, 'r'),
            (0o002, 'w'),
            (0o001, 'x'),
        ];
        FLAGS
            .iter()
            .map(|&(bit, letter)| {
                if attributes.raw() & bit != 0 {
                    letter
                } else {
                    '-'
                }
            })
            .collect()
    }

    pub fn set_attributes(path: &Path, attributes: Attributes) -> Result<(), VfsError> {
        use std::os::unix::fs::PermissionsExt;
        // Only the permission bits: the rest of st_mode says what kind of
        // thing this is, which a copy has already decided.
        let permissions = std::fs::Permissions::from_mode(attributes.raw() & 0o7777);
        Ok(std::fs::set_permissions(path, permissions)?)
    }

    /// The BSD "hidden" flag, which macOS sets on `~/Library` among others.
    ///
    /// Spelled here rather than taken from `libc`, because `libc` defines it
    /// for BSD targets only and the *rule* below has to compile — and be
    /// tested — on Linux. The value is stable BSD API (`chflags(2)`).
    const UF_HIDDEN: u32 = 0x8000;

    /// Whether an entry is hidden, given its name and its BSD flags.
    ///
    /// Pure, so the rule is testable on Linux: a leading dot hides an entry
    /// everywhere on Unix, and macOS *additionally* hides what carries
    /// [`UF_HIDDEN`] — Finder's convention, and a file manager that showed
    /// `~/Library` as ordinary would surprise a Mac user twice over.
    pub(super) fn hidden_from(name: &str, flags: u32) -> bool {
        name.starts_with('.') || flags & UF_HIDDEN != 0
    }

    /// Unix convention: a leading dot hides the entry.
    #[cfg(not(target_os = "macos"))]
    pub fn is_hidden(name: &str, _metadata: &std::fs::Metadata) -> bool {
        hidden_from(name, 0)
    }

    /// macOS: the dot, or the `UF_HIDDEN` flag only the metadata knows.
    #[cfg(target_os = "macos")]
    pub fn is_hidden(name: &str, metadata: &std::fs::Metadata) -> bool {
        use std::os::macos::fs::MetadataExt;
        hidden_from(name, metadata.st_flags())
    }

    /// Unix has a real root directory, so the generic code path handles it.
    pub fn root_entries() -> Option<Vec<Entry>> {
        None
    }

    /// Native and VFS paths coincide on Unix.
    pub fn from_std_path(path: &Path) -> VfsPath {
        VfsPath::new(&path.to_string_lossy())
    }

    pub fn home_dir() -> Option<PathBuf> {
        std::env::var_os("HOME").map(PathBuf::from)
    }

    /// Where the kernel lists what is mounted.
    #[cfg(not(target_os = "macos"))]
    const MOUNT_TABLE: &str = "/proc/self/mounts";

    /// The root, which is always worth a button whatever else is mounted.
    const ROOT_LABEL: &str = "/";

    /// Filesystem types that are the kernel talking to itself. None of them
    /// is a place a person navigates to, and a machine has dozens.
    #[cfg(not(target_os = "macos"))]
    const PSEUDO_FILESYSTEMS: [&str; 22] = [
        "autofs",
        "binfmt_misc",
        "bpf",
        "cgroup",
        "cgroup2",
        "configfs",
        "debugfs",
        "devpts",
        "devtmpfs",
        "efivarfs",
        "fusectl",
        "hugetlbfs",
        "mqueue",
        "nsfs",
        "overlay",
        "proc",
        "pstore",
        "ramfs",
        "securityfs",
        // Every snap on the machine is one of these, read-only, and a stock
        // Ubuntu desktop has twenty-six. They are application images rather
        // than places, and left in they push the actual disks off the end of
        // the drive bar — which is the failure the "a machine has dozens"
        // line above is about. A squashfs somebody loop-mounted to look
        // inside is still reachable by typing its path; the bar is a
        // shortcut list, not the only way in.
        "squashfs",
        "sysfs",
        "tracefs",
    ];

    /// These directories, **and everything below them**: kernel plumbing with
    /// a path, and `/run` in particular is full of it.
    ///
    /// Written without the trailing slash, and matched as "this path or a
    /// path inside it". With the slash they excluded only the children:
    /// `/run` is a `tmpfs` and is not in [`PSEUDO_FILESYSTEMS`], so it passed
    /// both filters and took a place in the drive bar — the first place, on
    /// the machine where it was found, which is what a test indexing the list
    /// positionally then walked into.
    ///
    /// `tmpfs` is deliberately *not* in the type list, which would have been
    /// the other way to exclude `/run`: `/tmp` is a `tmpfs` on many machines
    /// and is somewhere people very much do navigate to.
    #[cfg(not(target_os = "macos"))]
    const PSEUDO_ROOTS: [&str; 4] = ["/proc", "/sys", "/dev", "/run"];

    /// Reads the mount table.
    #[cfg(not(target_os = "macos"))]
    pub fn mount_points() -> Vec<Mount> {
        let table = std::fs::read_to_string(MOUNT_TABLE).unwrap_or_default();
        parse_mount_table(&table)
    }

    /// Turns the mount table into the buttons worth offering.
    ///
    /// Separated from reading it so the filter can be tested against a
    /// fixture: what a real machine happens to have mounted is not something
    /// a test should depend on, and the judgement about what counts as
    /// "somewhere a person goes" is the part worth reviewing.
    #[cfg(not(target_os = "macos"))]
    pub(super) fn parse_mount_table(table: &str) -> Vec<Mount> {
        let candidates = table.lines().filter_map(|line| {
            let mut fields = line.split_whitespace();
            let (Some(_device), Some(point), Some(kind)) =
                (fields.next(), fields.next(), fields.next())
            else {
                return None;
            };
            // The table escapes spaces as octal, which is the only escape
            // that turns up in practice.
            Some((point.replace("\\040", " "), kind.to_string()))
        });
        judge_mounts(candidates, &PSEUDO_FILESYSTEMS, &PSEUDO_ROOTS)
    }

    /// Which candidates deserve a button — the half both platforms share.
    ///
    /// The *lists* are per platform and the readers differ completely (a text
    /// table on Linux, `getfsstat` on macOS), but "drop the plumbing types,
    /// drop the plumbing roots and everything under them, dedupe, label by
    /// last component" is one judgement — and keeping it in one function is
    /// what lets the macOS lists be fixture-tested on a Linux box.
    pub(super) fn judge_mounts(
        candidates: impl Iterator<Item = (String, String)>,
        pseudo_filesystems: &[&str],
        pseudo_roots: &[&str],
    ) -> Vec<Mount> {
        let mut mounts = Vec::new();
        for (point, kind) in candidates {
            if pseudo_filesystems.contains(&kind.as_str()) {
                continue;
            }
            if pseudo_roots
                .iter()
                .any(|root| point == *root || point.starts_with(&format!("{root}/")))
            {
                continue;
            }
            let path = VfsPath::new(&point);
            let label = match path.file_name() {
                Some(name) => name.to_string(),
                None => ROOT_LABEL.to_string(),
            };
            if mounts.iter().any(|mount: &Mount| mount.path == path) {
                continue;
            }
            mounts.push(Mount { path, label });
        }
        mounts
    }

    /// Filesystem types that are macOS talking to itself.
    ///
    /// **Asserted from documentation, not from a running Mac** — the first
    /// thing to review when one exists. `devfs` is `/dev`; `autofs` is the
    /// automounter's placeholder mounts (`/System/Volumes/Data/home` and the
    /// `map -hosts` style entries).
    // `any(macos, test)` is the precise statement of who reads these: the
    // macOS `mount_points` at runtime, and the Linux-run fixture tests below,
    // which exist so the lists are reviewable without a Mac.
    #[cfg(any(target_os = "macos", test))]
    pub(super) const MACOS_PSEUDO_FILESYSTEMS: [&str; 2] = ["devfs", "autofs"];

    /// Directories that are plumbing on macOS, and everything below them.
    ///
    /// `/System/Volumes` holds the sealed-system split (`Preboot`, `VM`,
    /// `Update`, and the `Data` volume, which is already reachable as `/`
    /// through firmlinks — a second button for the same files would be a
    /// trap). `/private/var/vm` is swap. External disks land under
    /// `/Volumes/<name>`, which is exactly what the drive bar is for and is
    /// deliberately **not** listed here.
    #[cfg(any(target_os = "macos", test))]
    pub(super) const MACOS_PSEUDO_ROOTS: [&str; 3] = ["/dev", "/System/Volumes", "/private/var/vm"];

    /// The mounted filesystems, asked of the kernel.
    ///
    /// `getfsstat(2)` with `MNT_NOWAIT`: the cached list, not a fresh `statfs`
    /// of every mount — a network mount that has gone away must not hang the
    /// drive bar. Two calls, as the API intends: first a count, then the fill;
    /// a mount appearing between the two is cut off by the kernel, which
    /// reports how many it actually wrote.
    #[cfg(target_os = "macos")]
    pub fn mount_points() -> Vec<Mount> {
        let candidates = read_fsstat().into_iter();
        judge_mounts(candidates, &MACOS_PSEUDO_FILESYSTEMS, &MACOS_PSEUDO_ROOTS)
    }

    /// Every mount as `(mount point, filesystem type)`, decoded from the
    /// fixed-size C arrays `statfs` carries them in.
    #[cfg(target_os = "macos")]
    fn read_fsstat() -> Vec<(String, String)> {
        // SAFETY: the first call asks only for the count; the second hands
        // the kernel a buffer of exactly that many zeroed `statfs` records
        // and the true byte size, and trusts only as many records as the
        // kernel says it filled. The name fields are NUL-terminated by the
        // kernel within their fixed arrays.
        unsafe {
            let count = libc::getfsstat(std::ptr::null_mut(), 0, libc::MNT_NOWAIT);
            if count <= 0 {
                return Vec::new();
            }
            let mut stats = vec![std::mem::zeroed::<libc::statfs>(); count as usize];
            let bytes = std::mem::size_of_val(&stats[..]) as libc::c_int;
            let filled = libc::getfsstat(stats.as_mut_ptr(), bytes, libc::MNT_NOWAIT);
            if filled <= 0 {
                return Vec::new();
            }
            stats.truncate(filled as usize);
            stats
                .iter()
                .map(|stat| {
                    let text = |field: &[libc::c_char]| {
                        std::ffi::CStr::from_ptr(field.as_ptr())
                            .to_string_lossy()
                            .into_owned()
                    };
                    (text(&stat.f_mntonname), text(&stat.f_fstypename))
                })
                .collect()
        }
    }

    /// How much room the filesystem holding `path` has, and how much is left.
    ///
    /// One `statvfs` rather than running `df`: a process per directory step is
    /// not what a status line costs (`docs/performance.md`). `f_bavail` and
    /// not `f_bfree` — the first is what an unprivileged process may still
    /// write, the second includes the reserve only root may touch, and the
    /// question the status line answers is "will my copy fit".
    ///
    /// `None` when the path cannot be asked about at all: a filesystem that
    /// has gone is a status line with nothing in it, not a zero.
    pub fn space(path: &Path) -> Option<Space> {
        use std::os::unix::ffi::OsStrExt;

        let native = std::ffi::CString::new(path.as_os_str().as_bytes()).ok()?;
        // SAFETY: `statvfs` fills the struct or fails; the path is a valid
        // NUL-terminated C string for the length of the call.
        let stats = unsafe {
            let mut stats = std::mem::MaybeUninit::<libc::statvfs>::uninit();
            match libc::statvfs(native.as_ptr(), stats.as_mut_ptr()) {
                0 => stats.assume_init(),
                _ => return None,
            }
        };
        let block = stats.f_frsize as u64;
        Some(Space {
            free: stats.f_bavail as u64 * block,
            total: stats.f_blocks as u64 * block,
        })
    }

    /// Where per-user settings belong when `$XDG_CONFIG_HOME` says nothing:
    /// the freedesktop `~/.config` on Linux, `~/Library/Application Support`
    /// on macOS — each the place every other program on that machine uses.
    #[cfg(not(target_os = "macos"))]
    const CONFIG_FALLBACK: &str = ".config";
    #[cfg(target_os = "macos")]
    const CONFIG_FALLBACK: &str = "Library/Application Support";

    /// `$XDG_CONFIG_HOME` when set and absolute, else the platform's own
    /// place. The XDG override is honoured on macOS too: somebody who sets it
    /// there has said where they want their dotfiles, and the tests lean on
    /// it besides.
    pub fn config_dir() -> Option<PathBuf> {
        if let Some(configured) = std::env::var_os("XDG_CONFIG_HOME") {
            let path = PathBuf::from(configured);
            if path.is_absolute() {
                return Some(path);
            }
        }
        home_dir().map(|home| home.join(CONFIG_FALLBACK))
    }

    /// The freedesktop backend wraps the real `io::Error`, so unwrapping it
    /// keeps a failed trash in the same closed error set as every other call —
    /// a path that is already gone reports `NotFound`, not an opaque string.
    ///
    /// The arm carries the crate's own `cfg`: `Error::FileSystem` exists on
    /// freedesktop targets only, and `unix` also covers macOS.
    pub fn trash_error(err: trash::Error) -> VfsError {
        match err {
            #[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "android")))]
            trash::Error::FileSystem { source, .. } => VfsError::from(source),
            other => VfsError::Io(other.to_string()),
        }
    }
}

#[cfg(windows)]
mod imp {
    use std::os::windows::fs::MetadataExt;
    use std::time::SystemTime;

    use super::super::constants::SEPARATOR;
    use super::super::types::EntryKind;
    use super::*;

    /// `FILE_ATTRIBUTE_HIDDEN` from the Win32 API.
    const FILE_ATTRIBUTE_HIDDEN: u32 = 0x0000_0002;

    /// Drive letters probed when enumerating the synthetic root.
    const DRIVE_LETTERS: std::ops::RangeInclusive<char> = 'A'..='Z';

    /// Maps `/C:/Users/pirx` onto `C:\Users\pirx`.
    ///
    /// The first VFS component is the drive; `root_entries` guarantees the
    /// root itself is never routed here, so there is always a drive to take.
    pub fn to_std_path(path: &VfsPath) -> PathBuf {
        let mut components = path.components();
        let drive = components.next().unwrap_or_default();
        let mut native = format!("{drive}{}", std::path::MAIN_SEPARATOR);
        for component in components {
            native.push_str(component);
            native.push(std::path::MAIN_SEPARATOR);
        }
        PathBuf::from(native.trim_end_matches(std::path::MAIN_SEPARATOR))
    }

    /// The Win32 file attributes.
    pub fn attributes(metadata: &std::fs::Metadata) -> Attributes {
        Attributes::from_raw(metadata.file_attributes())
    }

    /// Windows has nowhere to put a Unix mode, and the raw value it does keep
    /// means something else entirely — so a zip made on Linux shows no
    /// attributes here rather than a row of nonsense flags.
    pub fn attributes_from_unix_mode(_mode: u32) -> Attributes {
        Attributes::default()
    }

    /// Nothing here is a Unix mode, so an archive written on Windows records
    /// none — rather than a Win32 attribute mask that would read on Linux as
    /// a permission nobody asked for.
    pub fn unix_mode_of(_attributes: Attributes) -> Option<u32> {
        None
    }

    /// `RHSA`, the letters Total Commander shows, with a dash where a flag is
    /// absent.
    pub fn render_attributes(attributes: Attributes) -> String {
        const FLAGS: [(u32, char); 4] = [
            (0x0000_0001, 'R'),
            (FILE_ATTRIBUTE_HIDDEN, 'H'),
            (0x0000_0004, 'S'),
            (0x0000_0020, 'A'),
        ];
        FLAGS
            .iter()
            .map(|&(bit, letter)| {
                if attributes.raw() & bit != 0 {
                    letter
                } else {
                    '-'
                }
            })
            .collect()
    }

    /// Only the read-only flag, which is all `std` can set here.
    ///
    /// Hidden, system and archive would need `SetFileAttributesW`, and this
    /// project has no Win32 binding. Recorded rather than silently dropped:
    /// see `docs/future-improvements.md`.
    pub fn set_attributes(path: &Path, attributes: Attributes) -> Result<(), VfsError> {
        const FILE_ATTRIBUTE_READONLY: u32 = 0x0000_0001;
        let mut permissions = std::fs::metadata(path)?.permissions();
        permissions.set_readonly(attributes.raw() & FILE_ATTRIBUTE_READONLY != 0);
        Ok(std::fs::set_permissions(path, permissions)?)
    }

    /// Windows convention: hidden is an attribute, not a naming rule. A
    /// dot-prefixed name such as `.gitignore` is a normal visible file here.
    pub fn is_hidden(_name: &str, metadata: &std::fs::Metadata) -> bool {
        metadata.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0
    }

    /// Windows has no single root, so the VFS root is the list of drives —
    /// which is exactly the drive selector the UI needs anyway.
    pub fn root_entries() -> Option<Vec<Entry>> {
        let drives = DRIVE_LETTERS
            .filter_map(|letter| {
                let name = format!("{letter}:");
                std::fs::metadata(format!("{name}{SEPARATOR}"))
                    .ok()
                    .map(|_| Entry {
                        name,
                        kind: EntryKind::Dir,
                        size: 0,
                        modified: SystemTime::UNIX_EPOCH,
                        attributes: Attributes::default(),
                        hidden: false,
                    })
            })
            .collect();
        Some(drives)
    }

    /// `C:\\Users\\pirx` becomes `/C:/Users/pirx`.
    pub fn from_std_path(path: &Path) -> VfsPath {
        VfsPath::new(
            &path
                .to_string_lossy()
                .replace(std::path::MAIN_SEPARATOR, "/"),
        )
    }

    pub fn home_dir() -> Option<PathBuf> {
        std::env::var_os("USERPROFILE").map(PathBuf::from)
    }

    /// The drives, which is exactly what `root_entries` already finds.
    pub fn mount_points() -> Vec<Mount> {
        root_entries()
            .unwrap_or_default()
            .into_iter()
            .map(|entry| Mount {
                path: VfsPath::root().child(&entry.name),
                label: entry.name,
            })
            .collect()
    }

    /// `%APPDATA%`, where per-user settings belong on Windows.
    /// Not answered on Windows yet.
    ///
    /// `GetDiskFreeSpaceExW` is the call, and it would need a Windows API
    /// crate this workspace does not otherwise want. A status line with no
    /// figure in it is honest; a made-up one is not
    /// (`docs/future-improvements.md`).
    pub fn space(_path: &Path) -> Option<Space> {
        None
    }

    pub fn config_dir() -> Option<PathBuf> {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .or_else(|| home_dir().map(|home| home.join("AppData").join("Roaming")))
    }

    /// The Windows backend reports Win32 status codes, which this layer does
    /// not model. Hand-mapping them would be a table of guesses; the original
    /// description survives in `Io`, which is what that variant is for.
    pub fn trash_error(err: trash::Error) -> VfsError {
        VfsError::Io(err.to_string())
    }
}

pub use imp::{
    attributes, attributes_from_unix_mode, config_dir, from_std_path, home_dir, is_hidden,
    mount_points, render_attributes, root_entries, set_attributes, space, to_std_path, trash_error,
    unix_mode_of,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_maps_onto_a_native_path_exactly_when_the_platform_has_one() {
        match root_entries() {
            // Unix: the generic path handles the root, so it must map.
            None => assert_eq!(to_std_path(&VfsPath::root()), PathBuf::from("/")),
            // Windows: the root is synthetic and lists drives instead.
            Some(drives) => assert!(drives.iter().all(|drive| drive.name.ends_with(':'))),
        }
    }

    #[test]
    fn a_native_path_survives_the_round_trip_through_the_vfs_path_space() {
        // The listing tests and the UI both map native paths into the VFS
        // path space; if that mapping is not an inverse, a pane opens
        // somewhere other than where it was told to.
        let native = home_dir().expect("a home directory");
        assert_eq!(to_std_path(&from_std_path(&native)), native);
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn the_mount_table_keeps_the_places_a_person_navigates_to() {
        // A fixture, not the running machine: what happens to be mounted
        // here is not something a test should depend on.
        let table = "\
/dev/vda1 / ext4 rw,relatime 0 0
proc /proc proc rw,nosuid 0 0
sysfs /sys sysfs rw,nosuid 0 0
tmpfs /run/user/1000 tmpfs rw,nosuid 0 0
/dev/vdb1 /mnt/backup ext4 rw,relatime 0 0
/dev/sdc1 /media/My\\040Stick vfat rw 0 0
cgroup2 /sys/fs/cgroup cgroup2 rw 0 0
/dev/vda1 / ext4 rw,relatime 0 0
";
        let mounts = imp::parse_mount_table(table);

        let points: Vec<&str> = mounts.iter().map(|m| m.path.as_str()).collect();
        assert_eq!(points, ["/", "/mnt/backup", "/media/My Stick"]);
        assert_eq!(mounts[0].label, "/", "the root labels itself");
        assert_eq!(mounts[2].label, "My Stick", "and an escaped space is one");
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn a_stock_ubuntu_desktop_offers_its_disks_and_nothing_else() {
        // The table a real machine reported, trimmed. Everything here passed
        // both filters once: `/run` because the prefixes carried a trailing
        // slash and so matched only its children, and the snaps because
        // `squashfs` was not in the type list. The drive bar was then
        // twenty-eight buttons with the two disks somewhere inside it, and
        // `/run` — not `/` — was the first.
        let table = "\
tmpfs /run tmpfs rw,nosuid,nodev 0 0
/dev/nvme0n1p2 / ext4 rw,relatime 0 0
/dev/loop1 /snap/bare/5 squashfs ro,nodev 0 0
/dev/loop2 /snap/core22/2411 squashfs ro,nodev 0 0
/dev/loop3 /snap/code/258 squashfs ro,nodev 0 0
tmpfs /run/user/1000 tmpfs rw,nosuid 0 0
/dev/nvme0n1p1 /boot/efi vfat rw,relatime 0 0
";
        let points: Vec<String> = imp::parse_mount_table(table)
            .iter()
            .map(|mount| mount.path.as_str().to_string())
            .collect();

        assert_eq!(points, ["/", "/boot/efi"]);
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn tmp_survives_even_when_it_is_a_tmpfs() {
        // The other way to have excluded `/run` was to call every `tmpfs`
        // plumbing. This is why that would have been wrong: `/tmp` is a
        // `tmpfs` on plenty of machines and is somewhere people go daily.
        let table = "\
/dev/vda1 / ext4 rw 0 0
tmpfs /tmp tmpfs rw,nosuid,nodev 0 0
tmpfs /run tmpfs rw,nosuid,nodev 0 0
";
        let points: Vec<String> = imp::parse_mount_table(table)
            .iter()
            .map(|mount| mount.path.as_str().to_string())
            .collect();

        assert_eq!(points, ["/", "/tmp"]);
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn a_mount_table_that_makes_no_sense_yields_no_buttons() {
        assert!(imp::parse_mount_table("").is_empty());
        assert!(imp::parse_mount_table("garbage\nalso garbage\n").is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn a_macs_mount_list_offers_the_root_and_the_external_disks() {
        // What `getfsstat` reports on a stock Mac, per documentation — the
        // reader itself can only run there, but the judgement over its output
        // is shared code and these are the macOS lists, reviewed here on
        // Linux because that is the box we have. `/System/Volumes/*` is the
        // sealed-system plumbing (`Data` is `/` again, through firmlinks — a
        // second button for the same files would be a trap); external disks
        // land under `/Volumes/<name>` and are the drive bar's whole point.
        let reported = [
            ("/", "apfs"),
            ("/dev", "devfs"),
            ("/System/Volumes/Preboot", "apfs"),
            ("/System/Volumes/VM", "apfs"),
            ("/System/Volumes/Update", "apfs"),
            ("/System/Volumes/Data", "apfs"),
            ("/System/Volumes/Data/home", "autofs"),
            ("/private/var/vm", "apfs"),
            ("/Volumes/Backup", "apfs"),
            ("/Volumes/My Stick", "msdos"),
        ];
        let mounts = imp::judge_mounts(
            reported
                .iter()
                .map(|(point, kind)| (point.to_string(), kind.to_string())),
            &imp::MACOS_PSEUDO_FILESYSTEMS,
            &imp::MACOS_PSEUDO_ROOTS,
        );

        let points: Vec<&str> = mounts.iter().map(|m| m.path.as_str()).collect();
        assert_eq!(points, ["/", "/Volumes/Backup", "/Volumes/My Stick"]);
        assert_eq!(mounts[0].label, "/", "the root labels itself");
        assert_eq!(
            mounts[2].label, "My Stick",
            "a space needs no unescaping here"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_dot_hides_everywhere_and_the_bsd_flag_hides_on_its_own() {
        // The rule behind `is_hidden`, pure so this box can test what only a
        // Mac can trigger: Finder hides `~/Library` with UF_HIDDEN (0x8000),
        // a flag Linux never sets — which is why the Linux branch passes 0.
        assert!(imp::hidden_from(".config", 0), "the dot, flags or not");
        assert!(imp::hidden_from("Library", 0x8000), "the flag alone");
        assert!(imp::hidden_from(".hidden", 0x8000), "both at once");
        assert!(!imp::hidden_from("Documents", 0), "neither");
        assert!(
            !imp::hidden_from("Documents", 0x4000),
            "a different flag is not this flag"
        );
    }

    #[cfg(unix)]
    #[test]
    fn unix_hides_dot_prefixed_names() {
        let metadata = std::fs::metadata(".").expect("cwd is readable");
        assert!(is_hidden(".config", &metadata));
        assert!(!is_hidden("config", &metadata));
    }

    #[cfg(windows)]
    #[test]
    fn windows_maps_the_first_component_onto_a_drive() {
        assert_eq!(
            to_std_path(&VfsPath::new("/C:/Users/pirx")),
            PathBuf::from("C:\\Users\\pirx")
        );
        assert_eq!(to_std_path(&VfsPath::new("/C:")), PathBuf::from("C:"));
    }
}
