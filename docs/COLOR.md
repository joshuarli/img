# Color, alpha, orientation and precision

Scored processing accepts eight-bit SDR samples. Untagged RGB and gray are interpreted as sRGB. PNG sRGB and supported CICP signals are honored. Valid CICP takes precedence over ICC, preventing an inactive profile from becoming active during conversion. Exact equality compares recognized required PNG color chunks. PNG gAMA without a stronger signal uses its declared power transfer function. Unrecognized chromaticities, recognized HDR signaling, unsupported profiles, and high precision are refused for scoring and resizing. [AVIF](AVIF.md) records the remaining decode-boundary defects.

ICC processing uses moxcms 0.8.1 (BSD-3-Clause OR Apache-2.0). Verified paths are RGB matrix profiles and gray profiles. Float transforms disable fixed-point preference and enable extended range. Scoring converts to linear sRGB directly and keeps out-of-gamut values; resizing and eight-bit sRGB output refuse them, rounding only numerical endpoint error within 0.00001 into the unit interval. Display P3 and Adobe RGB preparation is checked against independent LittleCMS float transforms into a linear-sRGB profile, including saturated out-of-gamut pixels. Exact PNG paths retain 16-bit samples and color profiles without requiring scored conversion.

Preparation applies EXIF orientation once and keeps exact eight-bit alpha separately. Metric compositing belongs to the scoring operation. Hidden RGB remains in the encoded image. Prepared pixels are never published as flattened composites.

Resizing uses fast_image_resize with bilinear filtering in linear light. Alpha is premultiplied before filtering and unpremultiplied afterward. The result is converted back through the source color interpretation and retains the source profile. Dimensions preserve aspect ratio and never enlarge. Stored orientation and its tags remain consistent until stripping is requested.

Stripping removes EXIF, XMP and private PNG chunks and retains required color chunks, including `cLLI` content-light metadata. Stripping physically normalizes orientation before removing tags. Exact sample preservation refuses nonidentity orientation. Tests cover all eight EXIF orientations and repeat preparation after encoding to catch double rotation.

Float color buffers and metric pyramids contribute to the working reservation. [Still-image limits](CODEC-BOUNDARY.md) own the ceilings, admission model and current metadata/comparison gaps.

The legacy library's `ApplyICC::new` now accepts ICC bytes and returns a parse result. It no longer exposes a LittleCMS profile type. The operation supports RGB and RGBA integer samples at 8 or 16 bits. Other color layouts fail explicitly. This constructor change follows removal of the native color backend. LittleCMS remains a development-only independent reference.

Some nominal sRGB ICC files contain quantized colorants whose floating-point
transform leaves the destination gamut near its endpoints. These are refused
rather than clipped. The pinned astronaut photograph exercises this case:
independent LittleCMS also produces values outside the supported interval.
Exact sample-preserving paths can retain that profile. Its refusal is recorded
as incomplete work in benchmark results, never as a fast successful encode.

Canonical sRGB and gamma transfer functions use libm 0.2.16, already present transitively. Platform powf differed by one float ULP for some eight-bit codes and moved a saved photographic score by 0.0117. Fixed float-bit cases and byte-identical prepared source/candidate dumps on macOS and Linux ARM64 verify the repair. Saved-score tolerance remains 0.0001; metric-oracle tolerance remains 0.05.

Unprofiled eight-bit sRGB preparation computes a 256-entry transfer table per image. It uses the same libm values without interpolation; all input codes and captured prepared pixels stay bit-identical. ICC and gamma transforms retain their floating-point paths. See [measured tuning](../BENCHMARKS.md#transfer-table-tuning).
