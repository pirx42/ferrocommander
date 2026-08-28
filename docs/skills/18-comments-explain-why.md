# Skill: Comments explain the "why", not the "what"

**When.** You are typing a comment into the code right now.

**Rule.** No comment that paraphrases the code line. Comments **only**
for:
- hidden invariants
- workarounds
- design decisions that would not be clear to the reader without context
- pointers to external issues / bugs

**Why.** The code says what it does; the comment should say why.
"Increments counter" is noise; "Retry counter — provides the jitter in
the reconnect flow (issue #412)" is context.

**How.**
- Default: no comment.
- Ask yourself: "Would a good reader understand this line on first
  read?" If yes → no comment. If no → minimal hint.
- Avoid multi-paragraph docstrings — one line is usually enough.
- Comments are not task communication ("used by X", "added for Y
  flow") — that belongs in the PR / commit body and otherwise rots in
  the code.

**Example.**
```ts
// YES — hidden invariant
// HM refs must be set before pass 2, otherwise NaN in resolveTree.
const hm = heatManagerRef.current;

// YES — workaround with justification
// iOS Safari: download.click() throttles the next ~700ms — see memory
// project_chimera_post_load_canvas_freeze.
showClipboardModal();

// NO — paraphrases
// Increment counter
counter++;

// NO — task context, rots
// Used by SettingsScreen after the 2026-04 refactor
function getThemeColor() { ... }
```

**Anti-patterns.**
- Banner comments above every function repeating the signature.
- Comments that repeat exactly what the variable name says.
- "TODO" comments without an issue reference or date.

**Related.**
- [30-document-the-why.md](30-document-the-why.md)
- [19-no-defensive-programming.md](19-no-defensive-programming.md)
