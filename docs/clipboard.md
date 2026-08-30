# The clipboard — `Ctrl+C`, `Ctrl+X`, `Ctrl+V`

← Parent: [CLAUDE.md](CLAUDE.md)

**The system clipboard, never a buffer of our own.** That is the whole design
decision, and it is what makes one implementation answer two requirements:
copying between the two panes and copying to and from Nautilus are the same
code, because both are "write the format other file managers read".

A second FerroCommander window is just another program as far as this is
concerned, and gets the same behaviour for free.

## What goes on it

Three formats, because they answer three different readers:

| Format | Who reads it | Carries |
|---|---|---|
| `x-special/gnome-copied-files` | Nautilus, Nemo, Thunar, Caja | the verb **and** the paths |
| `text/uri-list` | almost everything | the paths |
| plain text | a terminal, an editor | one path per line |

The first line of the GNOME payload is `copy` or `cut`, then one `file://`
URI per line. **That word is the only place a cut is recorded.**
`text/uri-list` has nowhere to put it, which is why a program offering only
the list is offering a copy — and why KDE needs a marker of its own, which
this does not yet write ([future-improvements.md](future-improvements.md)).

## What the encoding has to get right

The encoding is `tc-core::clipboard`, pure and tested there rather than in
the shell — it is a wire format other programs read, so it is worth testing
without a display server in the way.

- **Per component, not per path.** Encoding a whole path at once turns every
  `/` into `%2F` and produces a URI naming a single file with slashes in its
  name. A test says so directly.
- **A space is `%20`.** `my notes.txt` is an ordinary name and a URI carrying
  the space raw is one nothing can read back, us included.
- **Only `file:///`.** `trash://`, `smb://`, a bare path and
  `file://host/path` are refused rather than turned into a local path with
  the scheme removed — which would act on a file of that name that happens to
  exist here.
- **A payload with no verb yields nothing.** Guessing wrong in one direction
  deletes the source, so an intent that cannot be read is not guessed at.
- **One unreadable entry does not cost the others**, the same rule the
  operation engine follows for a source it cannot read.

## What paste does

It builds a `Job` and hands it to the queue — the same `Copy` and `Move` the
function keys use, with the same conflict questions and the same progress
window. Nothing new about copying is invented here; what is new is a *source
of paths* that did not come from a pane.

**A cut is a move, and the move is the engine's.** It copies, verifies, and
only then removes the source. Nothing in the clipboard code deletes anything,
which is what makes honouring another application's cut safe to do at all.

Reading is asynchronous, because the clipboard's owner is another process and
may take its time.

## The read is bounded, and a full buffer is refused

A clipboard is somebody else's data and its size is their choice, so the read
stops at a megabyte — tens of thousands of paths, and a file manager pasting
more than that has a different problem.

**A read that comes back exactly full is refused rather than used.** The call
reads *up to* the limit, so a payload that fits exactly and one that was cut
short look identical from here, and half a list is the worst thing to act on:
a copy would silently miss files, and a cut would move a subset and then clear
the clipboard holding the rest.

## Where it is refused

- **Inside an archive**, `Ctrl+C` and `Ctrl+X` say why: an entry has no
  operating-system path, so a URI naming one would point at a file on the
  disk that merely shares its name. The same reason `F4` and `Enter` are
  refused there ([archives.md](archives.md)).
- **Pasting into an archive** is refused before anything is read, since the
  backend is read-only.

## Windows

None of this applies there: the clipboard is `CF_HDROP` with a separate
preferred-drop-effect. The module is freedesktop-shaped and says so rather
than pretending to be portable.
