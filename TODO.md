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


### RIMG-10 · P2 · AV1 bitstream HDR metadata escapes AVIF validation and inspection

- **Area:** AVIF metadata boundary
- **Where:** src/still/avif.rs:8-9, src/still/avif.rs:27; src/still.rs:438-447
- **Problem:** Processing rejects container HDR properties but never checks decoded Frame::content_light or Frame::mastering_display. Inspection also reports only container properties and transfer signaling. Proven with a valid 430-byte eight-bit BT.709/sRGB AVIF containing an AV1 HDR_CLL metadata OBU but no container content-light property. Insert bytes `2a 06 01 03 e8 01 90 80` immediately after the sequence-header OBU in the 422-byte RIMG-2 fixture, and add 8 to the enclosing mdat size and primary iloc extent length. The bytes encode maxCLL 1000 and maxFALL 400. Native FFprobe and the decoded frame report those values, while Rimage describe returns hdr_metadata=false and decode accepts the image. This proves a missed refusal/reporting boundary, not an HDR pixel-conversion error. Enabling film grain does not repair it.
- **Recommendation:** Apply the unsupported light/mastering metadata policy to the decoded AV1 frame as well as container properties. Inspect bitstream metadata through a bounded OBU header path so inspect can report it without decoding pixel planes. Require a grain-free bitstream-only fixture so an independent grain refusal cannot mask the missing HDR check. Add container-only content-light and equivalent mastering metadata cases, keeping the processing refusal and inspection result consistent.
- **Fix with:** os-boundaries
- **Effort:** M

