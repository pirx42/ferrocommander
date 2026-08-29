# ops/

← Parent: [../CLAUDE.md](../CLAUDE.md) ·
Topic doc: [docs/ops.md](../../../../docs/ops.md)

The file-operation engine: what F5, F6, F7, F8 and Alt+F5 actually do.

| File | |
|---|---|
| `mod.rs` | `Job`, `run`, and the executors for a transfer, a removal and a pack |
| `plan.rs` | the scan: one source becomes an `Item` of `Task`s, with totals |
| `queue.rs` | the worker thread and the channels the shell awaits |
| `conflict.rs` | the question, the answer, and "apply to all" |
| `progress.rs` | the event stream and the `Report` |
| `cancel.rs` | the token every long loop checks |
| `constants.rs` | buffer sizes and the refusal messages |

## Rules that hold here

- **Scan, then execute.** The scan is what gives the progress bar an honest
  total; the one exception is a same-store move, which tries its `rename`
  first because the scan would be the whole cost.
- **`source_fs` and `target_fs` are two backends**, and a rule written for one
  filesystem asks `Store` before it assumes they are the same one.
- **Nothing here knows a file format.** A pack drives a
  [`Packer`](../archive/CLAUDE.md); the format comes from the archive's name.
- **A failure is per item, never per job.** One unreadable subdirectory must
  not cost the tree, and a move never deletes a source it could not fully
  copy.
