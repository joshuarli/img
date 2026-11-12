# RIMG-8: Define in-place writer ownership

Status: draft

## Summary

In-place publication verifies source identity, then performs an unconditional rename. Another writer can replace the directory entry between those operations and lose its new contents. Repeating the identity check cannot make the replacement conditional.

The recommended contract requires exclusive writer ownership while an in-place job runs. Serialize cooperating Rimage processes from source open through commit, retain identity checks, and document that external concurrent edits are unsupported. This is a proposed contract change, not a guarantee the current implementation provides.

## Motivation

`Source::verify` in `src/publication.rs` compares the opened source and current directory entry with the saved identity. `publish_cancellable` then calls `renameat`. A deterministic probe using the unchanged publication module inserted an editor's atomic save between those calls. Publication returned success and erased the editor's new bytes.

The ordinary no-clobber destination path does not share this race. In-place replacement needs an explicit ownership contract or a protocol that preserves displaced content.

## Design

Under the recommended contract, a job owns its logical destination before opening the source and until publication and cleanup finish. The ownership key must survive replacement of the source inode. Locking only the old file descriptor does not serialize a later process that opens the new inode.

Use one descriptor-relative coordination protocol for all cooperating Rimage writers. Preserve the existing native-path, symlink and alias checks. Specify lock ordering for multiple destinations, cancellation while waiting, and stale-state behavior before choosing the lock representation. A process exit must release live ownership without deleting another process's state.

After ownership is acquired, open and read the source, produce the verified bytes, and perform the existing identity check immediately before publication. Report a detected external edit as a conflict. The check remains useful but does not establish safety against an uncooperative writer after it completes.

The CLI documentation must state the exclusive-writer requirement. Sibling output remains the default. Do not describe cooperative locking as protection from arbitrary editors, external renames, or writes through an already-open descriptor.

Regression checks must force two cooperating jobs to overlap across source open and commit. Assert that the second job reads the committed source or reports a conflict. Keep the deterministic final-check race as evidence of the unsupported external-writer boundary. Preserve cancellation, no-clobber, symlink, staging-cleanup and post-commit durability tests.

## Alternatives considered

| Alternative | Consequence |
| --- | --- |
| Repeat `stat` immediately before `renameat` | Narrows the window but leaves the same lost-update race. |
| Lock the opened source inode | Does not coordinate later opens after an atomic replacement and does not stop uncooperative writers. |
| Preserve displaced content with a platform exchange/recovery protocol | Can support stronger recovery, but needs a validated macOS/Linux design for directory durability, crash states, concurrent third writes and open-descriptor mutation. A naive rollback can itself erase a newer write. |
| Disable in-place output | Keeps sibling publication safe while avoiding a concurrency promise the implementation cannot satisfy. Removes a useful explicit workflow. |

## Open questions

1. **Which writers must be protected?** Recommend exclusive ownership with cooperating Rimage processes. If arbitrary external editors must be protected, disable in-place until a recoverable replacement protocol is designed and validated.
2. **Should a competing Rimage job wait or fail?** Recommend an immediate conflict for predictable automation. Any future waiting mode must be bounded and cancellable, and acquire ownership before the authoritative source read.

## Decision

Pending owner decision. No runtime change is approved by this draft. Once accepted and implemented, move the supported guarantee into `docs/CLI.md` and `docs/CODEC-BOUNDARY.md`, close RIMG-8, and delete this RFC.
