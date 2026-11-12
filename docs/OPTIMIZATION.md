# Verified output selection

`optimizer::process` buffers one verified output and performs no filesystem I/O.
Requests select optimization or conversion, score/direct-quality/exact fidelity,
metadata policy, effort, and optional aspect-preserving resize. Lossless resize
is refused. Optimize requires strictly fewer bytes; conversion permits growth.

Fast, balanced and best allow 8, 24 and 64 encodes in total. Search tries exact
PNG and WebP first where eligible, then a fixed round-robin grid of 95, 50, 0, 100, 75,
and 25 across compatible formats. Every eligible format receives an attempt.
After the complete grid, deterministic midpoint refinement prioritizes gaps
near each format's smallest qualifying evaluated result. It never infers
fidelity from quality or assumes monotonic scores. Failed attempts count.
Equal sizes prefer higher measured score, then JPEG, PNG, WebP and AVIF; exact
ties preserve evaluation order. Original-byte identity is a separate option.

Each attempt records settings, size, actual decoded score, encode wall time,
and failure. Diagnostic encode times are not benchmark evidence. Only one
candidate is encoded per image at a time. Reference metric pyramids are reused.
Direct quality also decodes and validates the bytes but adds no score minimum.
Lossless compares expanded samples, depth, hidden RGB, alpha, ICC and orientation.
Same-format lossless JPEG copies bytes; cross-format JPEG and AVIF lossless fail.

Keep mode verifies ICC, EXIF, XMP and orientation after encoding. Cross-format
PNG ancillary losses are listed individually. Required PNG color signaling
without a preservable ICC profile prevents cross-format conversion. Unknown
unsafe-to-copy and layout-sensitive PNG metadata require explicit strip before
re-encoding. Keep can still choose compliant original bytes. Strip fallback
is allowed only after checking the source container has no private chunks;
source AVIF identity under strip is conservatively ineligible.

Source fallback is reported unchanged, never as an encoded candidate or an
unmeasured successful search. A source representation that cannot satisfy a
requested transformation cannot be selected. If no qualifying output exists,
processing fails before publication. The caller uses the shared atomic publisher.
