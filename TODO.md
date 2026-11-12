Prefix: RIMG
Next: 11

Priorities are assumed from impact and likelihood. Evidence was reproduced against `ba5f682175a746e80f351a6908b555fa7d0ce09c`; CI failures are outside this review.
### RIMG-4 · P2 · Compressed PNG text bypasses the decoded-metadata budget

- **Area:** Untrusted-input resource bounds
- **Where:** src/still/png.rs:18-21; src/still.rs:349, src/still.rs:362-364, src/still.rs:435; pinned zenpng 0.1.4 probe and ancillary text inflation
- **Problem:** The raw PNG walker counts compressed chunk bytes. The generic probe then expands and retains text before the post-decode check, which counts only ICC, EXIF and XMP. Inspection uses the same expanding probe. Proven with a valid 1x1 RGB8 PNG containing 16 zTXt chunks: keywords Comment0 through Comment15, compression method 0, and each text equal to zlib.compress(b"A" * 1048576, 9). The complete file is 17,067 bytes but contains 16,777,350 expanded keyword/text bytes. Both describe and decode accept it despite MAX_METADATA_BYTES=8,388,608. A 64-chunk, 68,091-byte version expands to 67,109,430 bytes and CLI inspect also succeeds. The pinned decoder caps individual text chunks, not their cumulative allocation. No deliberate OOM test was run; the accepted quota bypass is directly proved.
- **Recommendation:** Enforce the cumulative expanded ancillary budget before metadata-expanding probe or decode allocation, including describe/inspect. Bound inflation as bytes are produced, or skip unused text expansion while preserving bounded raw chunks where required. Include compressed text alongside ICC, EXIF and XMP. A post-probe rejection is too late. Use the small 16-chunk fixture as a regression without stressing host memory.
- **Fix with:** security
- **Effort:** M


### RIMG-5 · P2 · AVIF-to-PNG conversion retains a YUV matrix for RGB samples

- **Area:** Cross-format color metadata
- **Where:** src/still/avif.rs:49-74; src/still.rs:374-384; src/still/color.rs:69; src/optimizer.rs:101-115
- **Problem:** The AVIF adapter converts YUV to full-range RGB but stores the original matrix/range in metadata.cicp, which the PNG encoder copies verbatim. PNG requires matrix coefficients 0. Proven with the external BT.709/sRGB/full-range matrix-1 AVIF recipe in RIMG-2 without film grain: public lossless PNG encoding emits cICP [1,13,1,1], and preparing that PNG fails with "PNG CICP signaling is unsupported". The real CLI `rimage convert plain-srgb.avif --format png --lossless --effort fast -o plain.png --json` exits 3 with "no qualifying output: exact decoded samples differ" and publishes nothing. The required-color comparison sees the newly emitted invalid PNG chunk. This is independent of the film-grain defect.
- **Recommendation:** Normalize coding-dependent CICP when source planes become full-range RGB. Keep source container matrix/range separate from metadata describing current samples. Preserve CICP authority over any inactive ICC profile while producing valid RGB PNG signaling. Add an external AVIF-to-PNG case that succeeds losslessly and asserts cICP [1,13,0,1], with named identity, BT.709/BT.601 and range cases at the conversion boundary.
- **Fix with:** make-invalid-states-unrepresentable
- **Effort:** M


### RIMG-6 · P2 · Compare admits the candidate without accounting for its pixels

- **Area:** Comparison memory admission
- **Where:** src/runtime.rs:192-198; src/perceptual.rs:71-80
- **Problem:** Comparison reserves the reference pixel area plus compressed byte lengths, then fully decodes and prepares the candidate before checking dimensions. A highly compressed larger candidate can exceed the worker budget before returning an ordinary mismatch. Proven by invoking the unchanged runtime compare function with a 262,144-byte injected budget, an 8x8 reference of 72 bytes and a 256x256 candidate of 564 bytes. Admission computes 17,020 bytes and the call reaches input.score "score dimensions differ", rather than input.limit. Applying the same 256-bytes-per-pixel model to the candidate would require 16,777,852 bytes. No OOM claim or stress run is needed for the demonstrated missing admission check.
- **Recommendation:** Inspect both inputs and their orientation-correct dimensions before pixel preparation. Reject incompatible dimensions early and reserve all simultaneously live reference/candidate buffers through one accounting path. Test a small reference and larger compressed candidate under an injected tight budget, and preserve valid comparisons whose stored dimensions differ only because of orientation.
- **Fix with:** single-source-of-truth
- **Effort:** M


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

