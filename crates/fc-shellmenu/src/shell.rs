//! The conversation with the shell, in the order it happens.
//!
//! 1. A path becomes a PIDL (`SHParseDisplayName`), and the PIDL splits into
//!    its folder and its last part (`SHBindToParent`), because the shell
//!    hands out menus per *folder*: "the menu for these children of yours".
//! 2. The folder is asked for an `IContextMenu` over those children — or,
//!    for the empty space, its view is asked for the background object.
//! 3. The menu fills a Win32 `HMENU` (`QueryContextMenu`), which is what a
//!    person can look at and what the tests count.
//! 4. The `HMENU` is shown as a popup owned by a hidden window of ours
//!    (`TrackPopupMenuEx`), whose procedure forwards the messages an
//!    owner-drawn submenu — *Send to*, *Open with* — needs to draw itself.
//! 5. The chosen id goes back to the shell (`InvokeCommand`), and the shell
//!    does whatever the verb means. Nothing here knows what that was; the
//!    directory watcher notices.

use std::cell::RefCell;
use std::path::Path;

use windows::core::{Interface, PCSTR, PCWSTR, PSTR};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::System::Com::{
    CoInitializeEx, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::Common::ITEMIDLIST;
use windows::Win32::UI::Shell::{
    IContextMenu, IContextMenu3, ILFree, IShellFolder, IShellView, SHBindToParent,
    SHGetDesktopFolder, SHParseDisplayName, CMF_EXPLORE, CMF_NORMAL, CMIC_MASK_PTINVOKE,
    CMINVOKECOMMANDINFO, CMINVOKECOMMANDINFOEX, GCS_VERBW, SEE_MASK_UNICODE, SVGIO_BACKGROUND,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow, GetCursorPos,
    GetMenuItemCount, GetMenuItemID, PostMessageW, RegisterClassW, SetForegroundWindow,
    TrackPopupMenuEx, HMENU, SW_SHOWNORMAL, TPM_LEFTALIGN, TPM_RETURNCMD, TPM_TOPALIGN,
    WINDOW_EX_STYLE, WM_DRAWITEM, WM_INITMENUPOPUP, WM_MEASUREITEM, WM_MENUCHAR, WM_NULL,
    WNDCLASSW, WS_POPUP,
};

/// The first command id handed to the shell, and the last. Ids below the
/// first are ours to give to menu items of our own — there are none — and
/// the shell numbers its verbs from here; `0x7FFF` is the largest a menu
/// item id can be.
const FIRST_COMMAND: u32 = 1;
const LAST_COMMAND: u32 = 0x7FFF;

/// `CMIC_MASK_UNICODE`, which `shobjidl_core.h` defines as this very flag
/// and the bindings only carry under its `ShellExecute` name. It says the
/// wide members of the invocation are the ones to read.
const CMIC_MASK_UNICODE: u32 = SEE_MASK_UNICODE;

/// How many UTF-16 units a verb's name may hold. Verbs are short words —
/// `open`, `delete`, `properties` — and the shell truncates rather than
/// overruns.
const VERB_CAPACITY: usize = 64;

/// The class of the hidden window that owns the popup.
const OWNER_CLASS: PCWSTR = windows::core::w!("FerroCommanderShellMenu");

/// What went wrong, as the shell said it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub String);

impl From<windows::core::Error> for Error {
    fn from(error: windows::core::Error) -> Self {
        Error(error.to_string())
    }
}

/// Where the popup goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Where {
    /// Under the pointer, wherever it is — a menu the mouse asked for.
    Pointer,
    /// At a point of the owner window's client area — a menu the keyboard
    /// asked for, placed at the row it is about.
    Client { x: i32, y: i32 },
}

/// A context menu the shell built, filled into an `HMENU` and not yet shown.
///
/// Holding it holds the `IContextMenu` and the menu handle; dropping it
/// destroys the handle. The two are one value because a filled `HMENU` is
/// meaningless without the object that filled it — the ids in it are that
/// object's to interpret.
pub struct Menu {
    context: IContextMenu,
    handle: HMENU,
}

impl Menu {
    /// The menu Explorer would show for these items, which must all be
    /// children of one folder — the shell's own rule, and the reason a
    /// branch view asks for its cursor row alone.
    pub fn for_items(paths: &[&Path]) -> Result<Menu, Error> {
        let Some((first, rest)) = paths.split_first() else {
            return Err(Error("no items to build a menu for".to_string()));
        };
        initialise_com();
        // Every full PIDL is kept until the menu is built: the child PIDLs
        // `SHBindToParent` hands back point into them.
        let mut owned = Vec::with_capacity(paths.len());
        let mut children = Vec::with_capacity(paths.len());
        let folder: IShellFolder = unsafe {
            let pidl = parse(first)?;
            let mut child = std::ptr::null_mut();
            let folder = SHBindToParent(pidl, Some(&mut child))?;
            owned.push(Pidl(pidl));
            children.push(child as *const ITEMIDLIST);
            folder
        };
        for path in rest {
            unsafe {
                let pidl = parse(path)?;
                let mut child = std::ptr::null_mut();
                let _: IShellFolder = SHBindToParent(pidl, Some(&mut child))?;
                owned.push(Pidl(pidl));
                children.push(child as *const ITEMIDLIST);
            }
        }
        let context: IContextMenu =
            unsafe { folder.GetUIObjectOf(HWND::default(), &children, None)? };
        Menu::filled(context)
    }

    /// The menu Explorer shows on the empty space of `folder` — *New*,
    /// *Paste*, *Properties*: the folder's view's background object.
    pub fn for_background(folder: &Path) -> Result<Menu, Error> {
        initialise_com();
        let context: IContextMenu = unsafe {
            let pidl = Pidl(parse(folder)?);
            let desktop = SHGetDesktopFolder()?;
            let shell_folder: IShellFolder = desktop.BindToObject(pidl.0, None)?;
            let view: IShellView = shell_folder.CreateViewObject(HWND::default())?;
            view.GetItemObject(SVGIO_BACKGROUND)?
        };
        Menu::filled(context)
    }

    fn filled(context: IContextMenu) -> Result<Menu, Error> {
        let handle = unsafe { CreatePopupMenu()? };
        let menu = Menu { context, handle };
        unsafe {
            menu.context.QueryContextMenu(
                menu.handle,
                0,
                FIRST_COMMAND,
                LAST_COMMAND,
                CMF_NORMAL | CMF_EXPLORE,
            )?;
        }
        Ok(menu)
    }

    /// How many items the shell put in the menu, separators and submenus
    /// included.
    pub fn len(&self) -> usize {
        unsafe { GetMenuItemCount(self.handle).max(0) as usize }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The verbs behind the top-level items, by name — `open`, `delete`,
    /// `properties` — for an item that has one. Separators and submenus
    /// carry no id and are skipped; a verb the handler declines to name is
    /// skipped too, which is allowed and common.
    pub fn verbs(&self) -> Vec<String> {
        let mut verbs = Vec::new();
        for position in 0..self.len() as i32 {
            let id = unsafe { GetMenuItemID(self.handle, position) };
            if id == u32::MAX || id < FIRST_COMMAND {
                continue;
            }
            let mut name = [0u16; VERB_CAPACITY];
            let named = unsafe {
                self.context.GetCommandString(
                    (id - FIRST_COMMAND) as usize,
                    GCS_VERBW,
                    None,
                    PSTR(name.as_mut_ptr().cast()),
                    VERB_CAPACITY as u32,
                )
            };
            if named.is_ok() {
                let end = name
                    .iter()
                    .position(|&unit| unit == 0)
                    .unwrap_or(name.len());
                verbs.push(String::from_utf16_lossy(&name[..end]));
            }
        }
        verbs
    }

    /// Shows the menu and carries out what was chosen, if anything.
    ///
    /// `owner` is the application window's handle, for the shell to parent
    /// its own dialogs — *Properties* — on. The popup itself is owned by a
    /// hidden window of ours, so its messages have a procedure to go to
    /// without touching GTK's; see [`owner_procedure`].
    ///
    /// Modal: this returns when the menu is gone, which is what a popup is.
    /// The GTK main loop does not run meanwhile — a job's progress bar holds
    /// still for as long as the menu is up, as Explorer's own window does.
    pub fn track(self, owner: isize, at: Where) -> Result<bool, Error> {
        let owner = HWND(owner);
        let point = match at {
            Where::Pointer => unsafe {
                let mut point = POINT::default();
                GetCursorPos(&mut point)?;
                point
            },
            Where::Client { x, y } => unsafe {
                let mut point = POINT { x, y };
                let _ = ClientToScreen(owner, &mut point);
                point
            },
        };
        let helper = OwnerWindow::create()?;
        // Whichever the handler implements: the newer one is asked first,
        // because it is the one that can answer `WM_MENUCHAR`.
        let forward: Option<IContextMenu3> = self.context.cast().ok();
        FORWARD.with(|slot| *slot.borrow_mut() = forward);
        let chosen = unsafe {
            // The popup dismisses on a click elsewhere only while its owner
            // is the foreground window — the documented tray-icon dance.
            let _ = SetForegroundWindow(helper.0);
            let chosen = TrackPopupMenuEx(
                self.handle,
                (TPM_RETURNCMD | TPM_LEFTALIGN | TPM_TOPALIGN).0,
                point.x,
                point.y,
                helper.0,
                None,
            );
            let _ = PostMessageW(helper.0, WM_NULL, WPARAM(0), LPARAM(0));
            chosen.0 as u32
        };
        FORWARD.with(|slot| *slot.borrow_mut() = None);
        if chosen < FIRST_COMMAND {
            return Ok(false);
        }
        let verb = (chosen - FIRST_COMMAND) as usize;
        let info = CMINVOKECOMMANDINFOEX {
            cbSize: std::mem::size_of::<CMINVOKECOMMANDINFOEX>() as u32,
            fMask: CMIC_MASK_UNICODE | CMIC_MASK_PTINVOKE,
            hwnd: owner,
            // An id rather than a name, in both the ANSI and the wide slot:
            // `MAKEINTRESOURCE`, which is what the shell reads a small
            // integer where a string pointer goes as.
            lpVerb: PCSTR(verb as *const u8),
            lpVerbW: PCWSTR(verb as *const u16),
            nShow: SW_SHOWNORMAL.0,
            ptInvoke: point,
            ..Default::default()
        };
        unsafe {
            self.context.InvokeCommand(
                &info as *const CMINVOKECOMMANDINFOEX as *const CMINVOKECOMMANDINFO,
            )?;
        }
        Ok(true)
    }
}

impl Drop for Menu {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyMenu(self.handle);
        }
    }
}

/// A full PIDL the shell allocated, freed when dropped.
struct Pidl(*mut ITEMIDLIST);

impl Drop for Pidl {
    fn drop(&mut self) {
        unsafe { ILFree(Some(self.0)) };
    }
}

/// `path` as the shell names it.
unsafe fn parse(path: &Path) -> Result<*mut ITEMIDLIST, Error> {
    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let mut pidl = std::ptr::null_mut();
    SHParseDisplayName(PCWSTR(wide.as_ptr()), None, &mut pidl, 0, None)?;
    Ok(pidl)
}

use std::os::windows::ffi::OsStrExt;

/// COM, on this thread, the way the shell wants it: a single-threaded
/// apartment. GTK's main thread has one already — GDK initialises OLE for
/// drag and drop — and asking again is answered with "already", which is
/// fine; a test thread has none, and this is what gives it one.
fn initialise_com() {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE);
    }
}

thread_local! {
    /// The handler the popup's owner forwards menu messages to while a menu
    /// is up. Thread-local because the interface is apartment-bound and the
    /// window procedure has no other way to find it.
    static FORWARD: RefCell<Option<IContextMenu3>> = const { RefCell::new(None) };
}

/// The hidden window that owns the popup, for the life of one menu.
struct OwnerWindow(HWND);

impl OwnerWindow {
    fn create() -> Result<OwnerWindow, Error> {
        unsafe {
            let instance = HINSTANCE(GetModuleHandleW(None)?.0);
            let class = WNDCLASSW {
                lpfnWndProc: Some(owner_procedure),
                hInstance: instance,
                lpszClassName: OWNER_CLASS,
                ..Default::default()
            };
            // Registered once per process; the second registration fails
            // with "already exists", which is the state wanted.
            let _ = RegisterClassW(&class);
            let handle = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                OWNER_CLASS,
                PCWSTR::null(),
                WS_POPUP,
                0,
                0,
                0,
                0,
                HWND::default(),
                HMENU::default(),
                instance,
                None,
            );
            if handle == HWND::default() {
                return Err(Error(
                    "the popup's owner window could not be created".to_string(),
                ));
            }
            Ok(OwnerWindow(handle))
        }
    }
}

impl Drop for OwnerWindow {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.0);
        }
    }
}

/// The owner window's procedure: the four messages an owner-drawn submenu
/// needs go to the handler, everything else to the default.
///
/// *Send to* and *Open with* draw their own items — icons beside names —
/// and they do it by being asked to, through these messages, which arrive
/// at the popup's owner. A menu shown without forwarding them opens those
/// submenus empty.
unsafe extern "system" fn owner_procedure(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if matches!(
        message,
        WM_INITMENUPOPUP | WM_DRAWITEM | WM_MEASUREITEM | WM_MENUCHAR
    ) {
        let handled = FORWARD.with(|slot| {
            slot.borrow().as_ref().map(|handler| {
                let mut result = LRESULT(0);
                let forwarded = handler.HandleMenuMsg2(message, wparam, lparam, Some(&mut result));
                forwarded.map(|_| result)
            })
        });
        if let Some(Ok(result)) = handled {
            return result;
        }
    }
    DefWindowProcW(window, message, wparam, lparam)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Everything up to the popup, on the machine that can: a real file, a
    /// real folder, the shell's own menu for each, and the verb that has to
    /// be in one of them. No window is shown and nothing is invoked.
    #[test]
    fn the_shell_builds_a_menu_for_a_file_and_it_can_be_deleted() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a file.txt");
        std::fs::write(&file, "x").unwrap();

        let menu = Menu::for_items(&[file.as_path()]).expect("the shell answers for a file");

        assert!(
            !menu.is_empty(),
            "the shell offered nothing for a text file"
        );
        let verbs = menu.verbs();
        assert!(
            verbs.iter().any(|verb| verb.eq_ignore_ascii_case("delete")),
            "no delete verb among {verbs:?}"
        );
    }

    #[test]
    fn two_files_of_one_folder_share_a_menu() {
        let dir = tempfile::tempdir().unwrap();
        let first = dir.path().join("first.txt");
        let second = dir.path().join("second.txt");
        std::fs::write(&first, "x").unwrap();
        std::fs::write(&second, "y").unwrap();

        let menu = Menu::for_items(&[first.as_path(), second.as_path()])
            .expect("the shell answers for two children of one folder");

        assert!(!menu.is_empty());
    }

    #[test]
    fn the_background_of_a_folder_has_a_menu_of_its_own() {
        let dir = tempfile::tempdir().unwrap();

        let menu = Menu::for_background(dir.path()).expect("the view answers for its background");

        assert!(!menu.is_empty(), "the folder's background offered nothing");
    }

    #[test]
    fn nothing_to_build_a_menu_for_is_an_error_not_a_menu() {
        assert!(Menu::for_items(&[]).is_err());
    }
}
