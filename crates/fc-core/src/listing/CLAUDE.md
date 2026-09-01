# listing/

← Parent: [../CLAUDE.md](../CLAUDE.md) ·
Topic doc: [docs/listing.md](../../../../docs/listing.md)

The model behind a pane: what a directory looks like once it is on screen.

| File | |
|---|---|
| `mod.rs` | `Listing` — entries, the `..` row, the cursor, the marks, the filter, and the threaded reads |
| `sort.rs` | `Sort`, and why directories are never mixed in with files |
| `name.rs` | reading a name: `split_name`, the one place an extension is split off, and `contains_ignoring_case`, the one match behind both the quick filter and type-ahead |
| `constants.rs` | the defaults a fresh listing starts from |

## Rules that hold here

- **It holds no filesystem, and names none.** A `Listing` is built from
  entries; who read them is the caller's business, which is what lets one show
  an archive. That includes the *opening*: `spawn` takes a closure and gives
  it a thread, so walking into an archive is `archive::spawn_enter` calling in
  rather than this module importing `ArchiveFs`.
- **A read happens on a worker thread**, and the answer arrives with the
  backend it was read from — because a step can change it.
- **`..` is prepended, never loaded**, so it cannot be sorted, filtered or
  marked. A root can be *given* one anyway, which is what an archive's root
  needs.
