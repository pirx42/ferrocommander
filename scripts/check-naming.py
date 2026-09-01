#!/usr/bin/env python3
"""Fails when `tc` reappears as anything but a name for Total Commander.

The crates were called `tc-core` and `tc-app` until 2026-09-01, from a
working title the project outgrew: `tc` in this repository's own names says
"Total Commander" about code that is not Total Commander. The owner's rule
is that **`tc` is allowed only when it names Total Commander itself** — the
program this one imitates.

The rename was 441 substitutions over 98 files, and nothing in the gate
would have noticed a single one coming back. That is how there were 441:
each was written by somebody following the spelling already in the file
next to them. So the rule is checked rather than remembered, the way
`check-links.py` checks links and the keymap tests check the bindings
table.

No toolchain, a second to run:

    python3 scripts/check-naming.py
"""

import re
import subprocess
import sys
from pathlib import Path

# The old crate names, in both spellings a Rust project writes them. These
# have no allowed use anywhere: the crates are `fc-core` and `fc-app`.
CRATE_NAMES = ("tc-core", "tc-app", "tc_core", "tc_app")

# Any other `tc-` or `tc_` prefixing a name. Broader than the four crate names
# on purpose — the rule is about the spelling, not two particular words, and a
# `tc-something` invented tomorrow is the same mistake with a new suffix.
#
# **The bare word is not searched**, in either case. The owner's rule allows
# `tc` for Total Commander itself, and that is how it gets written: `TC's own
# quirk` in prose, and the sentences that state this very rule — including the
# two below this check's own name in CLAUDE.md, which quote the spelling in
# order to forbid it. A check that failed on its own documentation would be
# read once and then worked around, which is worse than one that draws the
# line where the owner drew it: at the prefix.
NAME_PREFIX = re.compile(r"\btc[-_]\w")

# What may still carry the lowercase spelling, each with its reason.
ALLOWED = {
    # The design plan's filename and every link to it. `tc-clone` reads
    # "Total Commander clone", which is the allowed use — and renaming an
    # archived plan would rewrite a filename that four documents cite.
    "2026-08-28-tc-clone-design": "names the Total Commander clone the design was of",
    # The plan that removed the prefix has to quote what it removed.
    "drop-the-tc-prefix": "the rename plan, which is about the old name",
}


# How many findings are printed before the count speaks for the rest.
SHOWN = 12


def allowed(line: str) -> bool:
    return any(pattern in line for pattern in ALLOWED)


def main() -> int:
    files = subprocess.run(
        ["git", "ls-files"], capture_output=True, text=True, check=True
    ).stdout.split()

    findings = []
    for name in files:
        path = Path(name)
        # The plan document about the rename is the one file whose subject is
        # the old name; checking it would be checking a quotation.
        if any(pattern in name for pattern in ALLOWED):
            continue
        try:
            text = path.read_text()
        except (UnicodeDecodeError, FileNotFoundError, IsADirectoryError):
            continue
        for number, line in enumerate(text.splitlines(), start=1):
            if allowed(line):
                continue
            if any(crate in line for crate in CRATE_NAMES) or NAME_PREFIX.search(line):
                findings.append(f"{name}:{number}: {line.strip()}")

    if findings:
        print(f"{len(findings)} line(s) spell `tc` where it does not mean Total Commander:")
        # Capped, because the first run of this found 122 and a wall of them
        # says no more than a dozen does.
        for finding in findings[:SHOWN]:
            print(f"  {finding}")
        if len(findings) > SHOWN:
            print(f"  … and {len(findings) - SHOWN} more")
        print()
        print("The crates are `fc-core` and `fc-app`. `TC` in prose is how the")
        print("documents name Total Commander, and is what this check allows.")
        return 1

    print("0 stray `tc`")
    return 0


if __name__ == "__main__":
    sys.exit(main())
