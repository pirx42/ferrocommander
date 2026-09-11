# The application icon — and the icons the zip forgot to bring

Status: **Draft**, 2026-09-11 — awaiting the owner's go.

> the app has no icon in the os tab bar or if i switch between app using
> alt+tab

Three answers shaped this plan (2026-09-11): fix **both** gaps — the
application icon and the Windows zip's missing icon theme — **commit** the
rendered `.ico` and `.icns` rather than rasterise on each runner, and trim
the bundled theme to **the sizes and categories the rows use**.

## 1. What was measured, before anything was proposed

Every line of this section is a probe, not a reading.

**Linux is already right, and no code change can improve it.** GTK4's X11
backend sets **no `_NET_WM_ICON` at all**: with the window's `icon-name`
set to `folder` — an icon that certainly exists in the installed Adwaita —
`xprop` reports the property absent. The same with
`Window::set_default_icon_name`, and the same with the icon installed under
`$XDG_DATA_HOME/icons`. A window manager is expected to map `WM_CLASS` to a
desktop entry; ours reports `"ferrocommander"`, which is exactly the
`StartupWMClass=` in `packaging/st.rose.Ferrocommander.desktop`, whose
`Icon=st.rose.Ferrocommander` is what `package-deb.sh` installs into
`hicolor/scalable/apps`. So an installed `.deb` has its icon, a `cargo run`
build does not, and that is the correct behaviour rather than a bug.

**Windows has no icon because the `.exe` carries no icon resource.** That
is where a taskbar button and `Alt+Tab` get their picture from, and
`crates/fc-app/build.rs` embeds nothing — it stamps the build number and
stops. GTK cannot supply what the executable does not carry.

**macOS has none either:** `Info.plist` in `package-macos.sh` names no
`CFBundleIconFile`, and `Contents/Resources` holds no `.icns`.

**And the report nobody filed.** `package-windows.sh` ships no icon theme
and no gdk-pixbuf loaders, on the strength of this comment:

> the app names no icon and loads no image — a grep for `icon_name`,
> `IconTheme`, `Pixbuf` and `Image::` over `crates/fc-app/src` finds
> nothing

That was true when it was written and stopped being true on 2026-09-01,
when rows gained a leading icon ([ui-shell.md](../ui-shell.md)). Running
the app with an empty `XDG_DATA_DIRS` — which is what the zip amounts to —
every row draws the broken-image glyph instead of a folder or a file type.
The owner has not seen it because a machine that *built* the app has
MSYS2's own `share/icons` on the path; whoever unzips the published build
does not.

**The SVG rasterises correctly with ImageMagick alone** (no librsvg
needed): a 256 px PNG is faithful, and a six-size `.ico` (16…256) comes to
107 KB.

## 2. Decisions

1. **The rendered icons are committed, and a script in the repository is
   what renders them.** `scripts/render-icons.py` turns the one SVG into
   the `.ico`, the `.icns` and the PNG sizes; its output is committed
   beside the source. No runner grows a dependency, the build stays
   reproducible, and "re-render by hand" means running one script that is
   in the tree rather than remembering a command line (skill 68).
2. **The `.icns` is assembled by that script**, not by `iconutil`: the
   format is a magic word and a list of typed PNG chunks, the macOS runner
   is not where the assets are made, and a container format nobody can
   rebuild off the Mac is a format that rots.
3. **The `.exe` gets its icon through a resource compiled by `windres`**,
   from `build.rs`, linked with `cargo:rustc-link-arg-bins`. `windres`
   ships with the `mingw-w64-x86_64-toolchain` the Windows build already
   requires ([windows.md](../windows.md)), so this adds no dependency
   either. Windows uses the lowest-numbered icon resource in an executable,
   which is why the `.rc` names it `1`.
4. **Nothing is done to Linux**, because § 1 measured that there is nothing
   to do. That is written into the plan so the next person does not go
   looking.
5. **The bundled theme is Adwaita's `16x16` and its index, and nothing
   else.** The rows draw at `ROW_ICON_SIZE` — 16 px — so the PNG sizes are
   exactly what is asked for, and leaving the `scalable` SVGs out is what
   keeps **librsvg and its dependency tail** out of the zip. The four
   categories that directory holds — `places`, `mimetypes`, `devices`,
   `emblems` — are the ones a file list asks for.
6. **The packaging script checks the theme rather than claiming it.** The
   comment in § 1 was true when written and rotted silently; a check that
   `16x16/places/folder.png` arrived in the staged tree cannot rot, and
   fails the build if a future trim cuts too deep.

## 3. Phases

| # | | Commit |
|---|---|---|
| 1 | this plan | docs |
| 2 | `render-icons.py`, and the `.ico`/`.icns`/PNGs it makes | feat(packaging) |
| 3 | the `.exe` carries its icon | feat(windows) |
| 4 | the `.app` carries its icon | feat(macos) |
| 5 | the zip carries the icons the rows ask for | fix(windows) |
| 6 | docs + audit | docs / refactor |

## 4. Effort

Corrected per skill 45 (factor 0.10, holding across the last seven plans).

| Phase | Raw | Corrected |
|---|---|---|
| 2 assets + script | 1 d | 1 h |
| 3 Windows resource | 0.75 d | 0.75 h |
| 4 macOS bundle | 0.5 d | 0.5 h |
| 5 the theme in the zip | 1 d | 1 h |
| 6 docs + audit | 0.5 d | 0.5 h |

Under four hours, plus a full-gate run (~12 min) per phase commit — and a
Windows round and a macOS look that no figure here can size, because what
each platform does with an icon is not something this gate can see.

## 5. What would make this wrong

- **The `.exe` resource may not be the end of it.** Windows takes a
  taskbar icon from the window first and the executable second, and GDK
  registers the window class itself. If GDK sets a class icon of its own,
  the resource is ignored and the answer is `WM_SETICON` on the `HWND` —
  which costs nothing to reach, since `gdk4-win32` and `windows` are
  already dependencies for the shell menu. Only the owner's machine can
  say which it is, and phase 3 lands with that fallback named rather than
  written.
- **16 px PNGs still need gdk-pixbuf's PNG loader.** If MSYS2 builds it as
  a module rather than into the library, the loaders directory and its
  cache have to go into the zip as well — visible on the runner, invisible
  from here.
- **`.icns` written by hand.** Apple's own tool is not involved, so a
  wrong chunk type means an icon that silently does not appear. The
  committed PNGs are the fallback: `iconutil` on the runner can rebuild
  from them if the file is refused.
- **The trim is a rule, and rules rot** — which is the whole lesson of the
  comment this plan is fixing. Hence decision 6: the check is what keeps it
  honest, not the comment.
- **An icon nobody can see is worth nothing,** so every phase here ends in
  somebody looking at a taskbar. The gate can prove the `.ico` is in the
  binary and the `.icns` in the bundle; it cannot prove Windows likes
  either.
