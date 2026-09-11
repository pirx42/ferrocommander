# The application icon — and the icons the zip forgot to bring

Status: **Done**, 2026-09-11 — five phases, each behind a green gate.
What a taskbar makes of it is the owner's to see; outcome in § 6.

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
when rows gained a leading icon ([ui-shell.md](../../ui-shell.md)). Running
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
   the `.ico` and the `.icns`, and those two are committed beside the
   source. No runner grows a dependency, the build stays reproducible, and
   "re-render by hand" means running one script that is in the tree rather
   than remembering a command line (skill 68).

   **The intermediate PNGs are not committed**, which this plan first said
   they would be: nothing reads them. The `.deb` installs the SVG, the zip
   wants an icon *theme* rather than our art, and the `.icns` carries its
   own copies — so a committed PNG would be a file with no reader, which
   is what this project spends its comments arguing against. They are
   rendered into a temporary directory and assembled from there.
2. **The `.icns` is assembled by that script**, not by `iconutil`: the
   format is a magic word and a list of typed PNG chunks, the macOS runner
   is not where the assets are made, and a container format nobody can
   rebuild off the Mac is a format that rots.
3. **The `.exe` gets its icon through a resource compiled by `windres`**,
   from `build.rs`, linked with `cargo:rustc-link-arg-bins`. `windres`
   ships with the `mingw-w64-x86_64-toolchain` the Windows build already
   requires ([windows.md](../../windows.md)), so this adds no dependency
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
  wrong chunk type means an icon that silently does not appear. Answered
  as far as it can be from here: `--check` reads the file back and asserts
  every chunk's type against the pixel size of the PNG inside it, and the
  gate runs it. What that cannot prove is that Apple agrees with Apple's
  own documentation about which type means which size.
- **The trim is a rule, and rules rot** — which is the whole lesson of the
  comment this plan is fixing. Hence decision 6: the check is what keeps it
  honest, not the comment.
- **An icon nobody can see is worth nothing,** so every phase here ends in
  somebody looking at a taskbar. The gate can prove the `.ico` is in the
  binary and the `.icns` in the bundle; it cannot prove Windows likes
  either.

## 6. Outcome

**Everything in § 1 held**, and two estimates did not.

**The Windows resource was verified from Linux**, which § 5 did not expect.
With the MinGW cross toolchain installed here, the real build script ran
with the environment cargo gives it, compiled `ferrocommander.rc`, and the
object linked into a throwaway `.exe` whose PE resource directory came back
holding `RT_ICON` and `RT_GROUP_ICON`. So the mechanism is proven and only
the question § 5 named is open: whether GDK sets a window class icon that
takes precedence.

**The theme trim is 300 KB, not one to three megabytes.** Dropping
`scalable` drops librsvg with it, and 16 px PNGs turn out to need no
gdk-pixbuf loaders either — GDK reads PNG itself, which was checked by
running the app on this exact trim with `GDK_PIXBUF_MODULE_FILE` pointed at
an empty file. Every icon still drew. The second risk in § 5 therefore
never materialised.

**The intermediate PNGs are not committed** as decision 1 first said they
would be: nothing reads them, and a file with no reader is what this
project spends its comments arguing against. `render-icons.py` renders them
into a temporary directory and assembles from there.

**The audit found two things.** `write_ico` took a scratch directory it
never used, to look like its sibling — symmetry is not a reason for a
parameter. And the renderer was **not reproducible**: ImageMagick stamps
every PNG with the second it was made, so re-running the script rewrote
the `.icns` with 77 bytes of new timestamps and nothing else. That matters
more than it looks. A committed artifact whose provenance cannot be
re-derived is one nobody can check — "is this file what the SVG says it
should be" would have had no answer but trust, which is the opposite of
why it is committed. `-strip` settles it: two renders now agree byte for
byte, and the file lost 5 KB of metadata on the way.

Worth keeping for the next person: `png:exclude-chunk=date,time` is *not*
enough on ImageMagick 6 — it reaches the `tIME` chunk and leaves a pair of
`tEXt` timestamps behind. That was measured, twice, rather than assumed.

**What is still owed to a person.** Three of the four visible outcomes here
cannot be seen from this machine: the Windows taskbar and `Alt+Tab`, the
macOS Dock and Command-Tab, and the row icons in the published zip. The
gate proves the `.ico` is in the binary, the `.icns` in the bundle, and one
named icon in the staged tree — no more than that, which is why each is
recorded in [future-improvements.md](../../future-improvements.md) rather
than called done.
