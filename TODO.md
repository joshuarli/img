Prefix: RIMG
Next: 12

Priorities are assumed from impact and likelihood. Evidence was reproduced against `ba5f682175a746e80f351a6908b555fa7d0ce09c`; CI failures are outside this review.


### RIMG-11 · P3 · Make apply_srgb_profile independent of the ICC header clock

- **Area:** icc tests
- **Where:** src/operations/icc/tests.rs apply_srgb_profile
- **Problem:** The test compares the profile ApplySRGB writes with a second ColorProfile::new_srgb().encode() call. Each encode stamps the current time into the ICC header date-time field (bytes 24..36), so the two differ when the calls straddle a second boundary. Release run 43640729720 failed this way on x86_64-unknown-linux-musl (byte 35: 50 vs 51).
- **Recommendation:** Compare the profiles with the header date-time bytes masked, or encode with a fixed timestamp if the encoder allows it.
- **Fix with:** cargo test --locked --all-features --lib operations::icc::tests::apply_srgb_profile passes with the encoder clock forced across a second boundary.
- **Effort:** S

