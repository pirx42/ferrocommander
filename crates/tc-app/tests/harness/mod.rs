//! Driving the real binary with real key events, from a test.
//!
//! Unit tests cannot reach the layer where every bug in this project has so
//! far lived: the composition between GTK and the model. Phase 1 shipped
//! three of them past a green suite, and phase 2 added a fourth (a dialog
//! with no focused button) and a fifth (a symlink aborting a whole copy) that
//! only showed up when the program actually ran.
//!
//! So these tests start a private X server, launch the built binary on it,
//! send real key presses through the X server, and then assert on the
//! filesystem. Nothing is mocked; the only thing standing in for a person is
//! `xdotool`.
//!
//! **Requires `Xvfb` and `xdotool`** (Debian/Ubuntu: `xvfb`, `xdotool`). They
//! are part of the build prerequisites — see the root `CLAUDE.md`. A missing
//! tool fails the test with a message naming the package rather than being
//! skipped, because a test that quietly does not run is worse than no test.
//!
//! Three details cost real time to find and are the reason this harness
//! exists rather than a paragraph of advice:
//!
//! * A bare `Xvfb` has no window manager, so `xdotool windowactivate` fails
//!   with "_NET_ACTIVE_WINDOW not supported". `windowfocus` works.
//! * `xdotool key --window` sends `XSendEvent`, which GTK ignores. Plain
//!   `xdotool key` uses XTEST and arrives as a real key press.
//! * The app is single-instance. On a reachable session bus it hands off to
//!   the running instance and exits 0, so the test would drive nothing; an
//!   unreachable `DBUS_SESSION_BUS_ADDRESS` forces a private instance.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use tempfile::TempDir;

/// First X display number these tests use. High enough not to collide with a
/// desktop session on a developer's machine.
const DISPLAY_BASE: u32 = 90;

/// Big enough that no pane needs to scroll.
const SCREEN_WIDTH: u32 = 1400;
const SCREEN_HEIGHT: u32 = 900;
const SCREEN_DEPTH: u32 = 24;

/// How long the app may take to put its window up. Generous: the first frame
/// under software rendering is not quick.
const READY_TIMEOUT: Duration = Duration::from_secs(30);

/// How long an effect — a dialog appearing, a file arriving — may take.
const EFFECT_TIMEOUT: Duration = Duration::from_secs(15);

/// Gap between polls. Polling rather than sleeping a fixed amount is what
/// keeps these tests both fast and reliable.
const POLL: Duration = Duration::from_millis(50);

/// How long a freshly focused dialog is given to move the focus onto its
/// entry.
///
/// The one sleep in this harness, and it is here because there is genuinely
/// nothing to poll: the X server reports the *window* that has the focus, and
/// which widget inside it holds the focus is GTK's business alone. Typing
/// into the gap loses the first characters, which turned `*.txt` into `.txt`
/// and marked nothing — a failure that looked like a bug in the wildcard
/// matcher.
const FOCUS_SETTLE: Duration = Duration::from_millis(200);

/// What the main window's title starts with. The rest of it is the build
/// number and commit hash, which change with every commit — so this is the
/// part a window search can rely on, and `xdotool search --name` matches a
/// substring.
const MAIN_WINDOW: &str = "FerroCommander";

/// Where the app's own output goes, inside its private home.
const APP_LOG: &str = "app.log";

/// Every app gets its own display number, so nothing collides even when a
/// previous one is still shutting down.
static NEXT_DISPLAY: AtomicU32 = AtomicU32::new(DISPLAY_BASE);

/// Only one app runs at a time.
///
/// Cargo runs tests in parallel by default, and sixteen X servers with
/// sixteen GTK apps between them do not fit comfortably in a container: the
/// suite went from all-green to eight failures and back between runs, always
/// with apps dying at startup on a display that had just answered. None of
/// that says anything about the program under test. A suite that fails
/// randomly teaches people to ignore red, so these tests queue instead.
static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

/// A running app on a private X display, with a private home directory.
pub struct App {
    display: String,
    home: TempDir,
    xvfb: Child,
    app: Child,
    window: String,
    /// Set once the app has been shut down deliberately, so `Drop` does not
    /// go looking for processes that are already reaped.
    closed: bool,
    /// Held for the lifetime of the app, which is the test's lifetime.
    /// A poisoned lock is just a test that panicked earlier, which is no
    /// reason to fail every test after it.
    _turn: MutexGuard<'static, ()>,
}

impl App {
    /// Starts an X server and the built binary on it.
    ///
    /// `arrange` populates the home directory before the app is launched, so
    /// the panes show the fixture from their first frame.
    pub fn launch(arrange: impl FnOnce(&Path)) -> App {
        let home = TempDir::new().expect("a temporary home");
        arrange(home.path());
        App::start(home)
    }

    /// Starts an app on a home directory that already exists, which is how a
    /// test checks what the last run left behind.
    fn start(home: TempDir) -> App {
        let turn = ONE_AT_A_TIME
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        require("Xvfb", "xvfb");
        require("xdotool", "xdotool");

        let display = format!(":{}", NEXT_DISPLAY.fetch_add(1, Ordering::Relaxed));
        let xvfb_log = home.path().join("xvfb.log");
        let screen = format!("{SCREEN_WIDTH}x{SCREEN_HEIGHT}x{SCREEN_DEPTH}");
        let xvfb = Command::new("Xvfb")
            .args([&display, "-screen", "0", &screen])
            .stdout(Stdio::from(
                std::fs::File::create(&xvfb_log).expect("an xvfb log"),
            ))
            .stderr(Stdio::from(
                std::fs::File::options()
                    .append(true)
                    .open(&xvfb_log)
                    .expect("the xvfb log"),
            ))
            .spawn()
            .expect("Xvfb starts");

        // Wait for the server to accept connections. Launching into a display
        // that is not up yet leaves the app unable to connect, and it exits
        // before it draws anything — which reads as "no window appeared" and
        // is the wrong thing to go looking for.
        await_until(READY_TIMEOUT, || display_is_up(&display))
            .unwrap_or_else(|| panic!("X display {display} never came up"));

        // Kept rather than discarded: when the app dies at startup the reason
        // is the only useful thing there is, and "no window appeared" is not
        // it.
        let log = home.path().join(APP_LOG);
        let app = Command::new(env!("CARGO_BIN_EXE_tc-app"))
            .env("DISPLAY", &display)
            .env("HOME", home.path())
            // Otherwise this hands off to an already-running instance.
            .env("DBUS_SESSION_BUS_ADDRESS", "unix:path=/nope")
            .stdout(Stdio::from(
                std::fs::File::create(&log).expect("a log file"),
            ))
            .stderr(Stdio::from(
                std::fs::File::options()
                    .append(true)
                    .open(&log)
                    .expect("the log file"),
            ))
            .spawn()
            .expect("the binary starts");

        let mut app = App {
            display,
            home,
            xvfb,
            app,
            window: String::new(),
            closed: false,
            _turn: turn,
        };
        app.window = app.await_main_window().expect("the main window appears");
        app.focus_main();
        app
    }

    /// The main window's full title, build stamp and all.
    pub fn title(&self) -> String {
        let name = Command::new("xdotool")
            .env("DISPLAY", &self.display)
            .args(["getwindowname", &self.window])
            .output()
            .expect("xdotool reads the window name");
        String::from_utf8_lossy(&name.stdout).trim().to_string()
    }

    pub fn home(&self) -> &Path {
        self.home.path()
    }

    /// Closes the app and waits for it to go, so whatever it writes on the
    /// way out has been written.
    ///
    /// Takes the app by value: there is nothing to drive afterwards, and the
    /// point is to be able to start a second one on the same home directory.
    pub fn close(mut self) -> TempDir {
        // Ctrl+Q, not a kill: settings are written from GTK's close handler,
        // and a killed process never runs one. Closing the way a person does
        // is also the only way to test that the handler works at all.
        self.key("ctrl+q");
        let deadline = Instant::now() + EFFECT_TIMEOUT;
        while self.app.try_wait().ok().flatten().is_none() {
            if Instant::now() >= deadline {
                let _ = self.app.kill();
                break;
            }
            std::thread::sleep(POLL);
        }
        self.reap()
    }

    /// Kills the app outright, the way a crash, a `kill -9` or a lost session
    /// does: no close handler runs, so anything the app had not written yet
    /// is gone.
    ///
    /// Takes the app by value for the same reason `close` does — the point is
    /// to start a second one on the same home directory.
    pub fn kill(mut self) -> TempDir {
        let _ = self.app.kill();
        self.reap()
    }

    /// Waits for both processes to go and hands back the home directory.
    fn reap(&mut self) -> TempDir {
        let _ = self.app.wait();
        let _ = self.xvfb.kill();
        let _ = self.xvfb.wait();
        self.closed = true;
        std::mem::replace(&mut self.home, TempDir::new().expect("a placeholder"))
    }

    /// Starts a second app on a home directory an earlier one left behind.
    pub fn relaunch(home: TempDir) -> App {
        App::start(home)
    }

    pub fn path(&self, relative: &str) -> PathBuf {
        self.home.path().join(relative)
    }

    /// Sends one key press to whatever currently has focus.
    pub fn key(&self, key: &str) {
        self.xdotool(&["key", "--clearmodifiers", key]);
    }

    /// Sends several key presses in order.
    pub fn keys(&self, keys: &[&str]) {
        for key in keys {
            self.key(key);
        }
    }

    pub fn type_text(&self, text: &str) {
        // `--` before the text, or xdotool reads a leading dash as a flag of
        // its own and refuses: typing `-again` onto the end of a command line
        // failed with no hint that the text was the problem.
        self.xdotool(&["type", "--clearmodifiers", "--", text]);
    }

    /// Puts the keyboard focus back on the panes, and waits until it is
    /// really there.
    pub fn focus_main(&self) {
        self.focus_window(&self.window.clone(), MAIN_WINDOW);
    }

    /// Waits for a dialog whose title matches `title` and focuses it.
    ///
    /// Finding and focusing are retried together, because a dialog can close
    /// between the two — a job that finished while the test was looking, say.
    /// Panics if it never appears: a dialog that does not open is exactly the
    /// kind of failure these tests exist to catch.
    pub fn focus_dialog(&self, title: &str) {
        let deadline = Instant::now() + EFFECT_TIMEOUT;
        loop {
            if let Some(window) = self.find_window(title) {
                if self.try_focus(&window, title) {
                    std::thread::sleep(FOCUS_SETTLE);
                    return;
                }
            }
            assert!(
                Instant::now() < deadline,
                "no dialog titled {title:?} could be focused; open windows: {:?}",
                self.window_names()
            );
            std::thread::sleep(POLL);
        }
    }

    /// Focuses a window and waits for the X server to agree that it worked.
    fn focus_window(&self, window: &str, title: &str) {
        let deadline = Instant::now() + EFFECT_TIMEOUT;
        while !self.try_focus(window, title) {
            assert!(
                Instant::now() < deadline,
                "the focus never reached {title:?}; open windows: {:?}",
                self.window_names()
            );
            std::thread::sleep(POLL);
        }
    }

    /// Asks for the focus and reports whether it actually landed.
    ///
    /// Asking is not the same as getting it. Without this check a key press
    /// can reach the window that *used* to have focus — which is how typing
    /// a directory name and pressing Escape ended up in the main window,
    /// where both are unbound and silently do nothing, while the dialog sat
    /// there open.
    fn try_focus(&self, window: &str, title: &str) -> bool {
        if !self.try_xdotool(&["windowfocus", window]) {
            return false;
        }
        self.focused_window_name()
            .is_some_and(|focused| focused.contains(title))
    }

    /// The name of whatever currently has the keyboard focus.
    fn focused_window_name(&self) -> Option<String> {
        let focused = Command::new("xdotool")
            .env("DISPLAY", &self.display)
            .arg("getwindowfocus")
            .output()
            .ok()?;
        let id = String::from_utf8_lossy(&focused.stdout).trim().to_string();
        let name = Command::new("xdotool")
            .env("DISPLAY", &self.display)
            .args(["getwindowname", &id])
            .output()
            .ok()?;
        Some(String::from_utf8_lossy(&name.stdout).trim().to_string())
    }

    /// Whether a dialog matching `title` is open right now.
    ///
    /// Only meaningful for asserting that one did *not* open, after
    /// [`App::settle`]. To assert that one closed, use
    /// [`App::await_dialog_closed`] — GTK unmaps a window on its own
    /// schedule, and checking the instant after the key that dismissed it is
    /// a race the test will lose about a third of the time.
    pub fn has_dialog(&self, title: &str) -> bool {
        self.find_window(title).is_some()
    }

    /// Waits for a dialog to go away, which is what proves an answer was
    /// taken rather than dropped.
    pub fn await_dialog_closed(&self, title: &str) {
        let deadline = Instant::now() + EFFECT_TIMEOUT;
        while self.find_window(title).is_some() {
            assert!(
                Instant::now() < deadline,
                "the {title:?} dialog is still open"
            );
            std::thread::sleep(POLL);
        }
    }

    /// Every window title on this display, for a failure message that says
    /// what was there instead of what was wanted.
    fn window_names(&self) -> Vec<String> {
        let output = Command::new("xdotool")
            .env("DISPLAY", &self.display)
            .args(["search", "--name", "."])
            .output();
        let Ok(output) = output else {
            return Vec::new();
        };
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|id| {
                let name = Command::new("xdotool")
                    .env("DISPLAY", &self.display)
                    .args(["getwindowname", id])
                    .output()
                    .ok()?;
                Some(String::from_utf8_lossy(&name.stdout).trim().to_string())
            })
            .filter(|name| !name.is_empty())
            .collect()
    }

    /// Waits until `relative` exists below the home directory.
    pub fn await_exists(&self, relative: &str) {
        let path = self.path(relative);
        await_until(EFFECT_TIMEOUT, || path.exists())
            .unwrap_or_else(|| panic!("{relative} never appeared"));
    }

    /// Waits until `relative` is gone.
    pub fn await_gone(&self, relative: &str) {
        let path = self.path(relative);
        await_until(EFFECT_TIMEOUT, || !path.exists())
            .unwrap_or_else(|| panic!("{relative} is still there"));
    }

    /// Waits until `relative` holds exactly `contents`.
    pub fn await_contents(&self, relative: &str, contents: &str) {
        let path = self.path(relative);
        await_until(EFFECT_TIMEOUT, || {
            std::fs::read_to_string(&path).is_ok_and(|found| found == contents)
        })
        .unwrap_or_else(|| {
            panic!(
                "{relative} holds {:?}, not {contents:?}",
                std::fs::read_to_string(&path)
            )
        });
    }

    /// Waits until `relative` exists and mentions `needle`.
    ///
    /// What a test wants of the settings file: that a particular change has
    /// reached the disk, without pinning the whole serialized form.
    pub fn await_mentions(&self, relative: &str, needle: &str) {
        let path = self.path(relative);
        await_until(EFFECT_TIMEOUT, || {
            std::fs::read_to_string(&path).is_ok_and(|found| found.contains(needle))
        })
        .unwrap_or_else(|| {
            panic!(
                "{relative} never mentioned {needle:?}; it holds {:?}",
                std::fs::read_to_string(&path)
            )
        });
    }

    /// Gives a job that should do nothing time to prove it.
    ///
    /// Needed only for negative assertions: waiting for something to *not*
    /// happen has no event to poll for.
    pub fn settle(&self) {
        std::thread::sleep(Duration::from_millis(750));
    }

    /// Waits for the main window, giving up early if the app has already
    /// exited — otherwise a crash at startup costs the full timeout and
    /// reports the wrong problem.
    fn await_main_window(&mut self) -> Option<String> {
        let deadline = Instant::now() + READY_TIMEOUT;
        loop {
            if let Some(window) = self.find_window(MAIN_WINDOW) {
                return Some(window);
            }
            if let Ok(Some(status)) = self.app.try_wait() {
                let log =
                    std::fs::read_to_string(self.home.path().join(APP_LOG)).unwrap_or_default();
                let xvfb =
                    std::fs::read_to_string(self.home.path().join("xvfb.log")).unwrap_or_default();
                panic!(
                    "the app exited before showing a window on {}: {status}\n\
                     --- app ---\n{log}\n--- xvfb ---\n{xvfb}",
                    self.display
                );
            }
            if Instant::now() >= deadline {
                return None;
            }
            std::thread::sleep(POLL);
        }
    }

    fn await_window(&self, title: &str, timeout: Duration) -> Option<String> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(window) = self.find_window(title) {
                return Some(window);
            }
            if Instant::now() >= deadline {
                return None;
            }
            std::thread::sleep(POLL);
        }
    }

    fn find_window(&self, title: &str) -> Option<String> {
        let output = Command::new("xdotool")
            .env("DISPLAY", &self.display)
            .args(["search", "--name", title])
            .output()
            .ok()?;
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .map(str::to_string)
    }

    /// Resizes the main window, the way dragging its corner would.
    pub fn resize(&self, width: u32, height: u32) {
        let (width, height) = (width.to_string(), height.to_string());
        self.xdotool(&["windowsize", &self.window, &width, &height]);
    }

    /// Runs xdotool and insists it worked. For key presses, which have no
    /// reason to fail.
    fn xdotool(&self, args: &[&str]) {
        assert!(self.try_xdotool(args), "xdotool {args:?} failed");
    }

    /// Runs xdotool and reports whether it worked. For anything about a
    /// window, which may have gone.
    fn try_xdotool(&self, args: &[&str]) -> bool {
        let status = Command::new("xdotool")
            .env("DISPLAY", &self.display)
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect("xdotool runs");
        // GTK needs a moment to act on a key before the next one arrives.
        std::thread::sleep(POLL);
        status.success()
    }
}

impl Drop for App {
    fn drop(&mut self) {
        if self.closed {
            return;
        }
        // Kill the app first: it is the one holding the display open.
        let _ = self.app.kill();
        let _ = self.app.wait();
        let _ = self.xvfb.kill();
        let _ = self.xvfb.wait();
    }
}

/// Whether an X server on `display` is ready to be drawn on.
///
/// Not merely "does it answer": Xvfb accepts a connection slightly before its
/// screen is usable, and an app that connects in that window dies with
/// "Failed to open display". Requiring the geometry to come back at the size
/// that was asked for is a probe that cannot pass early.
///
/// `search` is no use here at all — "no windows yet" and "no display" are the
/// same exit status.
fn display_is_up(display: &str) -> bool {
    let Ok(output) = Command::new("xdotool")
        .env("DISPLAY", display)
        .arg("getdisplaygeometry")
        .stderr(Stdio::null())
        .output()
    else {
        return false;
    };
    let reported = String::from_utf8_lossy(&output.stdout);
    output.status.success() && reported.trim() == format!("{SCREEN_WIDTH} {SCREEN_HEIGHT}")
}

/// Polls `condition` until it holds or the time runs out.
fn await_until(timeout: Duration, condition: impl Fn() -> bool) -> Option<()> {
    let deadline = Instant::now() + timeout;
    loop {
        if condition() {
            return Some(());
        }
        if Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(POLL);
    }
}

/// Fails with something actionable when a tool is missing, rather than
/// leaving the test to die on a confusing `NotFound`.
fn require(binary: &str, package: &str) {
    if Command::new("which")
        .arg(binary)
        .stdout(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
    {
        return;
    }
    panic!("{binary} is needed for the UI tests — install the {package} package");
}
