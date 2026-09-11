#!/usr/bin/env python3
"""Renders the application icon into the two container formats Windows and
macOS read, from the one SVG that is the source.

Run it after changing `packaging/st.rose.Ferrocommander.svg`, and commit
what it writes:

    python3 scripts/render-icons.py

**The rendered files are committed rather than built on each runner.** The
alternative — rasterising during packaging — would put ImageMagick into
MSYS2 and librsvg into the macOS runner's Homebrew, two dependencies for an
image that changes about never. Committed, the build stays reproducible and
neither runner grows a tool; the cost is that this script has to be run by
hand, which is why it is a script in the repository rather than a command
line in a document (skill 68).

Linux is not served here: the `.deb` installs the SVG itself, because a
freedesktop icon theme takes scalable art and every renderer on that
platform reads it.

Needs ImageMagick, and nothing else — it rasterises this SVG faithfully
without librsvg, which was checked before this script was written.
"""

import shutil
import struct
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SOURCE = ROOT / "packaging" / "st.rose.Ferrocommander.svg"
ICO = ROOT / "packaging" / "st.rose.Ferrocommander.ico"
ICNS = ROOT / "packaging" / "st.rose.Ferrocommander.icns"

# What Windows shows the icon at, from a 16 px tree row to a 256 px tile in
# Explorer's largest view. Windows picks the nearest size and scales the
# rest, so a missing size is a blurry icon rather than none.
ICO_SIZES = (16, 32, 48, 64, 128, 256)

# The chunks an `.icns` is made of: a four-character type, and the pixel
# size of the PNG inside it. The pairs come from Apple's iconset naming —
# `icon_32x32@2x` is 64 px and is `ic12`, and so on — and a type carrying
# the wrong size is an icon macOS silently declines to draw, which is why
# they are written down as pairs rather than computed.
ICNS_CHUNKS = (
    ("icp4", 16),
    ("icp5", 32),
    ("icp6", 64),
    ("ic07", 128),
    ("ic08", 256),
    ("ic09", 512),
    ("ic10", 1024),
    ("ic11", 32),
    ("ic12", 64),
    ("ic13", 256),
    ("ic14", 512),
)

# The SVG's own coordinate system, which is what a density is relative to:
# rendering at `size * 72 / VIEWBOX` dots per inch gives exactly `size`
# pixels. Read from the file rather than assumed, so a redrawn icon on a
# different grid does not quietly come out at the wrong scale.
POINTS_PER_INCH = 72

# Drops everything ImageMagick would write about *when* rather than *what*:
# a `tIME` chunk and a pair of `tEXt` timestamps per image.
#
# Here so that rendering the same drawing twice gives the same bytes.
# Without it the `.icns` came out different on every run — eleven images,
# each with the second it was made — and an artifact that cannot be
# reproduced is one whose provenance nobody can check: "is this file what
# the SVG says it should be" would have no answer but trust. `png:exclude-
# chunk` is not enough on ImageMagick 6; it reaches `tIME` and leaves the
# `tEXt` pair, which was measured rather than assumed.
STRIP_METADATA = "-strip"


def viewbox_size() -> int:
    """The SVG's square extent, from its own `viewBox`."""
    text = SOURCE.read_text(encoding="utf-8")
    start = text.index('viewBox="') + len('viewBox="')
    _, _, width, height = text[start : text.index('"', start)].split()
    if width != height:
        raise SystemExit(f"{SOURCE.name}: the icon is not square ({width}x{height})")
    return int(float(width))


def magick() -> str:
    """ImageMagick's command, whichever major version is installed."""
    for name in ("magick", "convert"):
        if shutil.which(name):
            return name
    raise SystemExit("ImageMagick is needed to render the icon: apt install imagemagick")


def render(tool: str, extent: int, size: int, into: Path) -> Path:
    """One PNG of `size` pixels, rendered at that size rather than scaled
    down from a larger one — a 16 px icon resampled from 1024 px is mush."""
    out = into / f"{size}.png"
    subprocess.run(
        [
            tool,
            "-background", "none",
            "-density", str(size * POINTS_PER_INCH // extent),
            str(SOURCE),
            "-resize", f"{size}x{size}",
            STRIP_METADATA,
            str(out),
        ],
        check=True,
    )
    return out


def write_ico(tool: str, extent: int) -> None:
    """ImageMagick writes a multi-size `.ico` itself, from one source — so
    unlike the `.icns` there is nothing here to assemble."""
    subprocess.run(
        [
            tool,
            "-background", "none",
            "-density", str(max(ICO_SIZES) * POINTS_PER_INCH // extent),
            str(SOURCE),
            STRIP_METADATA,
            "-define", f"icon:auto-resize={','.join(str(s) for s in reversed(ICO_SIZES))}",
            str(ICO),
        ],
        check=True,
    )


def write_icns(tool: str, extent: int, into: Path) -> None:
    """An `.icns` is a magic word, a length, and typed chunks — so it is
    assembled here rather than by `iconutil`, which exists only on a Mac.
    An asset no one off the target platform can rebuild is an asset that
    rots the first time the art changes."""
    rendered = {size: render(tool, extent, size, into) for _, size in set(ICNS_CHUNKS)}
    chunks = b""
    for kind, size in ICNS_CHUNKS:
        png = rendered[size].read_bytes()
        chunks += kind.encode("ascii") + struct.pack(">I", len(png) + 8) + png
    ICNS.write_bytes(b"icns" + struct.pack(">I", len(chunks) + 8) + chunks)


def png_width(png: bytes) -> int:
    """The width an `IHDR` declares — where a PNG says how big it is."""
    if png[:8] != b"\x89PNG\r\n\x1a\n":
        raise SystemExit("a chunk of the .icns does not hold a PNG")
    return struct.unpack(">I", png[16:20])[0]


def check() -> None:
    """Reads the committed files back and says whether they hold what they
    are supposed to.

    Structure rather than bytes: re-rendering and comparing would make the
    gate depend on the ImageMagick version of whatever machine runs it, and
    a check that fails on a library upgrade teaches people to ignore it.
    What this catches is the failure that is otherwise silent — a truncated
    `.ico`, or an `.icns` chunk whose type and size disagree, which macOS
    answers by drawing no icon and saying nothing.

    What it cannot catch is the SVG being redrawn without this script being
    run. Nothing in a rendered file records which source it came from; the
    guard against that is that re-rendering is one command, in the tree,
    named in the packaging document.
    """
    for made in (ICO, ICNS):
        if not made.is_file():
            raise SystemExit(f"{made.relative_to(ROOT)} is missing: run {Path(__file__).name}")

    ico = ICO.read_bytes()
    reserved, kind, count = struct.unpack("<HHH", ico[:6])
    if (reserved, kind) != (0, 1):
        raise SystemExit(f"{ICO.name} is not an icon file")
    # A width byte of zero means 256: the field is one byte and the format
    # is older than the size.
    sizes = {ico[6 + entry * 16] or 256 for entry in range(count)}
    if sizes != set(ICO_SIZES):
        raise SystemExit(f"{ICO.name} holds {sorted(sizes)}, wanted {sorted(ICO_SIZES)}")

    icns = ICNS.read_bytes()
    if icns[:4] != b"icns":
        raise SystemExit(f"{ICNS.name} is not an icon set")
    if struct.unpack(">I", icns[4:8])[0] != len(icns):
        raise SystemExit(f"{ICNS.name} declares a length that is not its own")
    found = []
    at = 8
    while at < len(icns):
        kind = icns[at : at + 4].decode("ascii")
        length = struct.unpack(">I", icns[at + 4 : at + 8])[0]
        found.append((kind, png_width(icns[at + 8 : at + length])))
        at += length
    if found != list(ICNS_CHUNKS):
        raise SystemExit(f"{ICNS.name} holds {found}, wanted {list(ICNS_CHUNKS)}")

    print(f"  {ICO.name} and {ICNS.name} hold what they should")


def main(argv: list[str]) -> None:
    if argv[1:] == ["--check"]:
        return check()
    if argv[1:]:
        raise SystemExit(f"usage: {Path(__file__).name} [--check]")
    tool = magick()
    extent = viewbox_size()
    with tempfile.TemporaryDirectory() as scratch:
        write_ico(tool, extent)
        write_icns(tool, extent, Path(scratch))
    check()
    for made in (ICO, ICNS):
        print(f"  {made.relative_to(ROOT)}  {made.stat().st_size:>8} bytes")


if __name__ == "__main__":
    sys.exit(main(sys.argv))
