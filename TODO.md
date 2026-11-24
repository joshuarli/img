Prefix: RIMG
Next: 13

Priorities are assumed from impact and likelihood. Evidence was reproduced against `ba5f682175a746e80f351a6908b555fa7d0ce09c`; CI failures are outside this review.

### RIMG-11 · P3 · Make apply_srgb_profile independent of the ICC header clock

- **Area:** icc tests
- **Where:** src/operations/icc/tests.rs apply_srgb_profile
- **Problem:** The test compares the profile ApplySRGB writes with a second ColorProfile::new_srgb().encode() call. Each encode stamps the current time into the ICC header date-time field (bytes 24..36), so the two differ when the calls straddle a second boundary. Release run 43640729720 failed this way on x86_64-unknown-linux-musl (byte 35: 50 vs 51).
- **Recommendation:** Compare the profiles with the header date-time bytes masked, or encode with a fixed timestamp if the encoder allows it.
- **Fix with:** cargo test --locked --all-features --lib operations::icc::tests::apply_srgb_profile passes with the encoder clock forced across a second boundary.
- **Effort:** S

### RIMG-12 · P3 · Mutation-test the codec decode bodies with a decode fuzz harness

- **Area:** Tests
- **Where:** src/still/ codec modules (png, jpeg, avif, av1, avif_container); docs/test-measurements.md
- **Problem:** The deslop Tests step bounded mutation to the decode, limits, publication, runtime and exit contracts; the codec bodies hold about 1,400 mutants that no bounded run covers, and corrupt-input coverage stops at the crafted cases in tests/crafted_input.rs.
- **Recommendation:** Add a decode fuzz harness at StillImage::decode/describe with a corpus seeded from tests/fixtures, then run sharded cargo mutants over the codec bodies and disposition survivors.
- **Fix with:** test-audit
- **Effort:** L

