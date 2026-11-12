Prefix: RIMG
Next: 11

Priorities are assumed from impact and likelihood. Evidence was reproduced against `ba5f682175a746e80f351a6908b555fa7d0ce09c`; CI failures are outside this review.
### RIMG-8 · P2 · In-place rename can erase a source changed after verification

- **Area:** Filesystem publication concurrency
- **Where:** src/publication.rs:74-79, src/publication.rs:169-172; src/runtime.rs:185-186
- **Problem:** Source::verify and renameat are separate operations. A concurrent editor can atomically save a new inode after verification succeeds, then the unconditional rename replaces those newer bytes. A deterministic probe compiled the unchanged publication module and interposed only the final renameat to install the editor replacement in that window. Publication returned success and the editor bytes were lost. Controls rejected mutation before verification and preserved new-destination no-clobber, symlink rejection and unpublished-stage cleanup. The narrow race is proved; adding another stat cannot make rename conditional.
- **Recommendation:** Decide the supported writer-ownership contract in the linked draft RFC. The recommended scope requires exclusive writer ownership and serializes cooperating Rimage processes from source open through commit, while clearly excluding arbitrary concurrent external edits. If protection from uncooperative writers is required, design preservation/recovery of displaced content or disable in-place until that guarantee is established. Retain the deterministic race and existing publication controls.
- **Fix with:** os-boundaries
- **Effort:** M
- **Design:** docs/rfc/RIMG-8-in-place-concurrency.md


### RIMG-9 · P2 · A full stdout pipe prevents graceful signal termination

- **Area:** Cancellation and output lifecycle
- **Where:** src/runtime.rs:181, src/runtime.rs:210, src/runtime.rs:224, src/runtime.rs:255-261, src/runtime.rs:272-274
- **Problem:** The graceful handler sets cancellation atomics, but binary stdout uses blocking write_all and the main thread joins that worker before re-raising the signal. Proven in the real CLI with a noisy 512x512 PNG and `convert --format png --lossless -o - --quiet`, with stdout held open and undrained. FIONREAD and F_GETPIPE_SZ both showed the pipe full at 65,536 bytes. After SIGTERM the process remained alive for the two-second observation, then completed its 787,072-byte output and exited by SIGTERM only after the reader drained the pipe. Report writes are also blocking. A live downstream process that stops reading can prevent orderly termination without any active codec call.
- **Recommendation:** Make stdout publication and reporting cancellable while blocked, using bounded writes and an interruptible readiness wait. Check cancellation between writes, preserve ordinary broken-pipe classification, clean unpublished file state, and re-raise the original signal. Add a full but open pipe regression with a termination deadline; a closed-pipe test does not cover this state.
- **Fix with:** lifecycle-concurrency
- **Effort:** M


