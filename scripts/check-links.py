#!/usr/bin/env python3
"""Reports every relative Markdown link in the repository that goes nowhere.

Documentation rots by moving, not by being wrong: archiving a plan or renaming
a module breaks the links pointing at it and nothing complains. The phase 7
audit found eleven of them at once, which is why this exists rather than a
habit of checking by hand.

Not part of the green gate: it needs no toolchain and takes a second, so it is
run when documentation moves. `python3 scripts/check-links.py`, exit code 1 if
anything is broken.
"""

import os
import re
import sys

# Prose that looks like a path but is not one, and the build output.
IGNORED_PREFIXES = ("http", "mailto")
IGNORED_LINKS = {"plans/..."}
SKIPPED_DIRECTORIES = {"target", ".git"}
LINK = re.compile(r"\]\(([^)#][^)]*)\)")
# A link inside a code span is an *example* of a link, not one — the skills
# quote them to say what the convention is.
CODE_SPAN = re.compile(r"``.+?``|`[^`]*`", re.DOTALL)
FENCE = re.compile(r"^```.*?^```", re.DOTALL | re.MULTILINE)


def markdown_files(root: str):
    for directory, subdirectories, files in os.walk(root):
        subdirectories[:] = [d for d in subdirectories if d not in SKIPPED_DIRECTORIES]
        for name in files:
            if name.endswith(".md"):
                yield os.path.join(directory, name)


def broken(path: str):
    base = os.path.dirname(path)
    with open(path, encoding="utf-8") as handle:
        text = handle.read()
    text = CODE_SPAN.sub("", FENCE.sub("", text))
    for match in LINK.finditer(text):
        link = match.group(1).split("#")[0]
        if not link or link.startswith(IGNORED_PREFIXES) or link in IGNORED_LINKS:
            continue
        if not os.path.exists(os.path.normpath(os.path.join(base, link))):
            yield link


def main() -> int:
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    os.chdir(root)
    failures = 0
    for path in sorted(markdown_files(".")):
        for link in broken(path):
            print(f"{path}: {link}")
            failures += 1
    print(f"{failures} broken link(s)")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
