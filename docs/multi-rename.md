# Multi-rename — Ctrl+M

← Parent: [CLAUDE.md](CLAUDE.md)

`Ctrl+M` renames what is marked in the active pane by a rule, with a preview of
every result before anything runs. `Ctrl+Z` puts the last batch back.

## The preview *is* the rename

`rename::preview(rules, names)` is one pure function, and it is called twice:
once to draw the table the user reads, once to build the moves that run. A
preview computed differently from the thing it previews is worse than no
preview — it looks like a check and is not one. That is the reliability
requirement talking ([reliability.md](reliability.md)), for the operation it
most obviously applies to: a hundred files renamed by a rule nobody could
inspect first.

Because the engine is pure and lives in `fc-core`, the rules are table-tested
(`crates/fc-core/tests/rename.rs`) with no display server anywhere near them.

## The rules

| Placeholder | |
|---|---|
| `[N]` | the name without its extension |
| `[E]` | the extension, without the dot |
| `[C]` | a counter |

The default template is `[N].[E]` and the default counter start is `1`, so the
tool opens renaming nothing. A search-and-replace runs **over the result**, not
over the original — so it can reach text the template itself introduced.

The counter follows the order the names were given, which is the order the pane
shows. That is the only numbering anybody can predict by looking at the screen.

A trailing dot is trimmed: `[N].[E]` on a file with no extension would
otherwise produce `README.`, which is not what anyone meant by the template.

## What is refused, and why it is refused *here*

Three rows never become a rename, and each says so in the preview instead:

| Refusal | |
|---|---|
| `Empty` | the rules produced no name at all |
| `HasSeparator` | the name contains `/` or `\` — that is a move, and this tool renames |
| `Collides` | another row in the same batch wants the same name |

The collision check is against the whole batch, refused rows included: two
files both wanting a name neither may have is still two files that cannot both
have it, and flagging only the second would suggest the first was fine.

Refusing a collision **before** submitting matters more than it looks. The job
queue would also catch it — the second move would find the target occupied and
ask the conflict question — but that turns a rule the user could have fixed in
the preview into a dialog in the middle of a running batch. The end-to-end test
asserts the conflict dialog never opens, because the files survive either way
and only the absent dialog tells the two builds apart.

## Applying, and undoing

Each changed row is submitted as its own `Job::Move` with a
`Destination::Exact` through the ordinary queue ([ops.md](ops.md)), so a name
already taken **outside** the batch asks the conflict question it always asks,
and a failure is reported the way every other failure is.

Undo is one batch deep. Applying records the `(new, old)` **paths** — paths,
not names, because an undo is a rename back and has to know where the files
went. `Ctrl+Z` submits the reverse moves and clears the record.

The recorded names are not trusted. Each goes back through the same machinery,
so a file that something else has since moved or replaced asks the same
conflict question, and one that has gone is reported as a failure rather than
silently skipped.

A stack of undos would be a history feature, and the honest version of that is
a larger thing than one key — see
[future-improvements.md](future-improvements.md).

## Keyboard

Enter in any of the rule fields means Rename, the way Enter means OK in every
other dialog here; Escape closes the tool without renaming anything. The
template arrives selected, so typing replaces it.
