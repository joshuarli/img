Prefix: RIMG
Next: 26

Priorities are assumed from impact and likelihood. Evidence was reproduced against `ba5f682175a746e80f351a6908b555fa7d0ce09c`; CI failures are outside this review.

### RIMG-13 · P2 · Bench evidence tests fail on CI and the bytes budget is undisposed

- **Area:** tests · bench
- **Where:** tests/test_bench.py (Evidence); BENCH_DISPOSITIONS
- **Problem:** CI has failed on master since 2026-11-24 16:32 UTC. On run 45473216691 (master, 19:43) the Linux jobs failed `test_a_round_overlapping_a_build_is_discarded_with_its_disposition`; on PR #15 (an unrelated bootstrap/python.lock bump) macOS failed `test_batch_verifies_each_distinct_output_once` and `test_identical_outputs_are_verified_once_and_distinct_outputs_each`. The failing set changes between runs and platforms, which points at timing or ordering dependence. Both runs also print `VERDICT bytes inconclusive … 1 cases over 1.1 (1 undisposed): bad 1.2` with `UNDISPOSED bytes bad: add a reason under BENCH_DISPOSITIONS`.
- **Recommendation:** Reproduce the Evidence tests repeatedly on Linux and macOS, remove their dependence on wall-clock or ordering, and either fix the `bad` bytes case or record its disposition with a reason.
- **Fix with:** root-cause
- **Effort:** M

### RIMG-18 · P2 · Test source replacement identity after path substitution

- **Area:** publication tests
- **Where:** src/publication.rs Source::verify; tests/codec_boundary.rs
- **Problem:** A mutation replacing either identity mismatch with a requirement that both mismatch survived. A path can be replaced while the originally opened descriptor still identifies the original file, so the earlier equivalence rationale is unproven.
- **Recommendation:** Add an isolated replacement test that changes the named file while preserving the opened file and asserts rejection.
- **Fix with:** test-audit
- **Effort:** M

### RIMG-11 · P3 · Make apply_srgb_profile independent of the ICC header clock

- **Area:** icc tests
- **Where:** src/operations/icc/tests.rs apply_srgb_profile
- **Problem:** The test compares the profile ApplySRGB writes with a second ColorProfile::new_srgb().encode() call. Each encode stamps the current time into the ICC header date-time field (bytes 24..36), so the two differ when the calls straddle a second boundary. Release run 43640729720 failed this way on x86_64-unknown-linux-musl (byte 35: 50 vs 51).
- **Recommendation:** Compare the profiles with the header date-time bytes masked, or encode with a fixed timestamp if the encoder allows it.
- **Fix with:** cargo test --locked --all-features --lib operations::icc::tests::apply_srgb_profile passes with the encoder clock forced across a second boundary.
- **Effort:** S

### RIMG-12 · P3 · Mutation-test the codec decode bodies with a decode fuzz harness

- **Area:** Tests
- **Where:** src/still/ codec modules (png, jpeg, avif, av1, avif_container); tests/crafted_input.rs
- **Problem:** The bounded mutation trials did not cover all codec decode bodies, including the remaining JPEG marker-walker mutants, and corrupt-input coverage stops at the crafted cases in tests/crafted_input.rs.
- **Recommendation:** Add a decode fuzz harness at StillImage::decode/describe with a corpus seeded from tests/fixtures, then run sharded cargo mutants over the codec bodies and disposition survivors.
- **Fix with:** test-audit
- **Effort:** L

### RIMG-17 · P3 · Test directory stream descriptor cleanup

- **Area:** publication tests
- **Where:** src/publication.rs read_directory Entries::drop; tests/codec_boundary.rs
- **Problem:** A mutation removing directory stream cleanup survived because concurrent tests prevent reliable process-wide descriptor counting; the leak is untested.
- **Recommendation:** Exercise directory reads in an isolated process and assert descriptors are released.
- **Fix with:** test-audit
- **Effort:** M

### RIMG-19 · P3 · Mutation-test webpkit bitstream writers

- **Area:** webpkit tests
- **Where:** codecs/webpkit/src/container; codecs/webpkit/src/lossy; codecs/webpkit/src/lossless
- **Problem:** The webpkit review did not run mutation testing of the bitstream writers, leaving mutation survivors undiscovered.
- **Recommendation:** Run a bounded writer mutation trial and investigate surviving behavior changes.
- **Fix with:** test-audit
- **Effort:** L

### RIMG-24 · P3 · Resolve surviving publication mutations

- **Area:** publication tests
- **Where:** src/publication.rs create_parent, read_directory, publish_cancellable; tests/codec_boundary.rs; tests/cli.rs
- **Problem:** The mutation review left surviving create_parent errno and read_directory bound comparisons, and a publish_cancellable syscall-result mutation was accepted without a direct fault test. Their equivalence or coverage needs a reproducible disposition.
- **Recommendation:** Reproduce each survivor, prove equivalence where applicable, and add behavioral tests for any meaningful change.
- **Fix with:** test-audit
- **Effort:** M

### RIMG-25 · P3 · Cover oversized AVIF headers before decode allocation

- **Area:** avif tests
- **Where:** tests/crafted_input.rs; src/still/avif.rs; src/still/avif_container.rs
- **Problem:** No crafted oversized AVIF header test establishes that dimension limits reject input before plane allocation. The parser ignores ispe and takes frame dimensions from the AV1 sequence header, so an ispe-only fixture cannot prove the intended boundary.
- **Recommendation:** Craft an oversized AV1 sequence header in an AVIF fixture and assert the public limit error before plane allocation; assess ispe mismatch behavior separately.
- **Fix with:** test-audit
- **Effort:** M

### RIMG-15 · P4 · Resolve AVIF sniffing of short ftyp boxes

- **Area:** format detection
- **Where:** src/still.rs Format::sniff
- **Problem:** A 24-byte AVIF ftyp box alone is classified unsupported because sniffing requires bytes 8 through 31.
- **Recommendation:** Document the minimum sniff length or recognize brands within the declared box size, with a boundary test.
- **Fix with:** none
- **Effort:** S

### RIMG-16 · P4 · Measure branch coverage of critical contracts

- **Area:** test tooling
- **Where:** src/still.rs; src/publication.rs; src/runtime.rs; src/exit.rs
- **Problem:** The existing LLVM coverage instrumentation reports line and region coverage but no branch coverage, leaving branch coverage unmeasured.
- **Recommendation:** Find a compatible branch coverage tool and report critical-contract branches without treating the figure as a gate.
- **Fix with:** test-audit
- **Effort:** M

### RIMG-20 · P4 · Decide where to run the independent WebP oracle

- **Area:** webpkit tests
- **Where:** codecs/webpkit/Cargo.toml oracle; tests/webp_codec.rs
- **Problem:** Independent libwebp checks live in img rather than webpkit; the optional oracle feature and libwebp-sys dependency have no imported oracle implementation, and in-crate FFI would require an unsafe-code decision.
- **Recommendation:** Decide whether an independent test can run without expanding the unsafe grant; retain a working oracle or remove the unused feature and dependency with owner approval.
- **Fix with:** test-audit
- **Effort:** M

### RIMG-21 · P4 · Decide whether to retire unused img API and scratch examples

- **Area:** library api
- **Where:** src/limits.rs; src/error.rs; examples/scratch-*.rs
- **Problem:** Several exported limits and error helpers have no repository consumer, and seven scratch examples have no references; external consumers and reproduction needs were not settled.
- **Recommendation:** Assess consumers and reproduction needs before deciding whether to remove the unused surface and examples.
- **Fix with:** dead-code-archaeology
- **Effort:** M

### RIMG-22 · P4 · Decide whether to reduce webpkit internal API

- **Area:** webpkit api
- **Where:** codecs/webpkit/src/lib.rs __internals; codecs/webpkit/src/lossless; codecs/webpkit/src/lossy
- **Problem:** Unused-per-build decoder, animation, metadata, mux and configuration APIs remain exposed under __internals; removing them changes the imported upstream module surface.
- **Recommendation:** Assess consumers and decide with the owner whether to retain or remove the unused internal API.
- **Fix with:** dead-code-archaeology
- **Effort:** M

### RIMG-23 · P4 · Exercise codec encode error paths

- **Area:** codec tests
- **Where:** src/still/png.rs; src/still/avif.rs
- **Problem:** The coverage review left codec failure arms in the PNG and AVIF encode paths unexercised by its selected fixtures.
- **Recommendation:** Identify reachable encoder failure cases and test their observable errors with appropriate controlled inputs.
- **Fix with:** test-audit
- **Effort:** M
