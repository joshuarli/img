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


