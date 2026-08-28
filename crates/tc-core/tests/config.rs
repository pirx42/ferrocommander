//! Behavioral tests for the settings file.
//!
//! A file manager that refuses to start over its own settings is worse than
//! one that forgets where you were, so most of these are about what happens
//! when the file is wrong rather than when it is right.

use std::io::{Read, Write};
use std::sync::Mutex;
use std::time::SystemTime;

use tc_core::config::{self, PaneSettings, Settings};
use tc_core::listing::{Sort, SortKey, SortOrder};
use tc_core::vfs::{Attributes, Entry, LocalFs, VfsError, VfsPath, VirtualFs};
use tempfile::TempDir;

fn root() -> (TempDir, VfsPath) {
    let dir = TempDir::new().unwrap();
    let path = LocalFs::vfs_path(dir.path());
    (dir, path)
}

fn settings() -> Settings {
    let mut settings = Settings::default();
    settings.window.width = 1600;
    settings.window.height = 900;
    settings.active_pane = 1;
    let mut left = PaneSettings {
        directory: "/home/pirx/projects".to_string(),
        show_hidden: true,
        ..PaneSettings::default()
    };
    left.set_sort(Sort::new(SortKey::Size, SortOrder::Descending));
    settings.set_pane(0, left);
    settings.set_pane(
        1,
        PaneSettings {
            directory: "/mnt/backup".to_string(),
            ..PaneSettings::default()
        },
    );
    settings
}

fn write_config(root: &VfsPath, text: &str) {
    LocalFs.create_dir(&root.child(config::CONFIG_DIR)).unwrap();
    let mut file = LocalFs.create_file(&config::config_path(root)).unwrap();
    file.write_all(text.as_bytes()).unwrap();
}

#[test]
fn settings_survive_being_saved_and_read_back() {
    let (_dir, root) = root();
    let original = settings();

    config::save(&LocalFs, &root, &original).unwrap();
    let (loaded, complaint) = config::load(&LocalFs, &root);

    assert_eq!(loaded, original);
    assert_eq!(complaint, None);
}

#[test]
fn a_first_run_gets_the_defaults_and_says_nothing() {
    // No file is not a problem worth mentioning.
    let (_dir, root) = root();

    let (loaded, complaint) = config::load(&LocalFs, &root);

    assert_eq!(loaded, Settings::default());
    assert_eq!(complaint, None);
}

#[test]
fn a_corrupt_file_is_reported_once_and_replaced_by_the_defaults() {
    let (_dir, root) = root();
    write_config(&root, "this is not toml {{{");

    let (loaded, complaint) = config::load(&LocalFs, &root);

    assert_eq!(loaded, Settings::default());
    assert!(complaint.is_some(), "the user has to be told once");
}

#[test]
fn a_file_from_another_version_does_not_stop_the_program() {
    // deny_unknown_fields makes this a complaint rather than a silent
    // half-load, and the defaults are what starts up.
    let (_dir, root) = root();
    write_config(&root, "colour_scheme = \"midnight\"\n");

    let (loaded, complaint) = config::load(&LocalFs, &root);

    assert_eq!(loaded, Settings::default());
    assert!(complaint.is_some());
}

#[test]
fn saving_leaves_no_temporary_file_behind() {
    let (_dir, root) = root();

    config::save(&LocalFs, &root, &settings()).unwrap();

    let directory = root.child(config::CONFIG_DIR);
    let names: Vec<String> = LocalFs
        .read_dir(&directory)
        .unwrap()
        .iter()
        .map(|entry| entry.name.clone())
        .collect();
    assert_eq!(names, [config::CONFIG_FILE]);
}

#[test]
fn a_save_that_never_finishes_leaves_the_previous_settings_intact() {
    // The point of writing beside the real file and renaming: a crash
    // between the two leaves the old settings, not half of the new ones.
    let (_dir, root) = root();
    config::save(&LocalFs, &root, &settings()).unwrap();

    // Exactly what an interrupted save leaves behind.
    let temporary = root
        .child(config::CONFIG_DIR)
        .child(config::CONFIG_TEMP_FILE);
    LocalFs
        .create_file(&temporary)
        .unwrap()
        .write_all(b"half a fi")
        .unwrap();

    let (loaded, complaint) = config::load(&LocalFs, &root);
    assert_eq!(loaded, settings());
    assert_eq!(complaint, None);
}

#[test]
fn saving_creates_the_directory_it_needs() {
    // A first run has no config directory, and on many machines no config
    // root either.
    let (dir, _) = root();
    let nested = LocalFs::vfs_path(&dir.path().join("fresh"));

    config::save(&LocalFs, &nested, &settings()).unwrap();

    let (loaded, _) = config::load(&LocalFs, &nested);
    assert_eq!(loaded, settings());
}

#[test]
fn the_written_file_is_readable_by_a_person() {
    // It is a text file people are expected to edit, so the round trip is
    // not the only thing that matters.
    let (_dir, root) = root();
    config::save(&LocalFs, &root, &settings()).unwrap();

    let mut text = String::new();
    std::io::Read::read_to_string(
        &mut LocalFs.open_read(&config::config_path(&root)).unwrap(),
        &mut text,
    )
    .unwrap();

    assert!(text.contains("/home/pirx/projects"), "{text}");
    assert!(
        text.contains("size"),
        "the sort key is written by name: {text}"
    );
}

/// Records which paths were opened for writing and what was renamed.
struct Recording {
    created: Mutex<Vec<VfsPath>>,
    renamed: Mutex<Vec<(VfsPath, VfsPath)>>,
}

impl Recording {
    fn new() -> Self {
        Recording {
            created: Mutex::new(Vec::new()),
            renamed: Mutex::new(Vec::new()),
        }
    }
}

impl VirtualFs for Recording {
    fn read_dir(&self, path: &VfsPath) -> Result<Vec<Entry>, VfsError> {
        LocalFs.read_dir(path)
    }
    fn stat(&self, path: &VfsPath) -> Result<Entry, VfsError> {
        LocalFs.stat(path)
    }
    fn create_dir(&self, path: &VfsPath) -> Result<(), VfsError> {
        LocalFs.create_dir(path)
    }
    fn remove_dir(&self, path: &VfsPath) -> Result<(), VfsError> {
        LocalFs.remove_dir(path)
    }
    fn remove_file(&self, path: &VfsPath) -> Result<(), VfsError> {
        LocalFs.remove_file(path)
    }
    fn rename(&self, from: &VfsPath, to: &VfsPath) -> Result<(), VfsError> {
        self.renamed
            .lock()
            .unwrap()
            .push((from.clone(), to.clone()));
        LocalFs.rename(from, to)
    }
    fn open_read(&self, path: &VfsPath) -> Result<Box<dyn Read + Send>, VfsError> {
        LocalFs.open_read(path)
    }
    fn create_file(&self, path: &VfsPath) -> Result<Box<dyn Write + Send>, VfsError> {
        self.created.lock().unwrap().push(path.clone());
        LocalFs.create_file(path)
    }
    fn set_modified(&self, path: &VfsPath, time: SystemTime) -> Result<(), VfsError> {
        LocalFs.set_modified(path, time)
    }
    fn set_attributes(&self, path: &VfsPath, attributes: Attributes) -> Result<(), VfsError> {
        LocalFs.set_attributes(path, attributes)
    }
    fn trash(&self, path: &VfsPath) -> Result<(), VfsError> {
        LocalFs.trash(path)
    }
}

#[test]
fn the_settings_file_is_never_written_in_place() {
    // The invariant behind "atomic", and the only one a test can hold on to:
    // whatever happens, the real file is only ever replaced by a rename, so
    // a crash leaves the old file or the new one and never half of either.
    // Checking that a temporary file is absent afterwards does not catch
    // this — a direct write leaves none either.
    let (_dir, root) = root();
    let fs = Recording::new();

    config::save(&fs, &root, &settings()).unwrap();

    let target = config::config_path(&root);
    let created = fs.created.lock().unwrap().clone();
    assert!(
        !created.contains(&target),
        "the settings file was opened for writing directly: {created:?}"
    );
    assert_eq!(created.len(), 1, "exactly one file was written");
    assert_eq!(
        *fs.renamed.lock().unwrap(),
        vec![(created[0].clone(), target)],
        "and it was renamed over the real one"
    );
}
