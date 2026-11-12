# Color, alpha, orientation and precision

Scored processing accepts eight-bit SDR samples. Untagged RGB and gray are interpreted as sRGB. PNG sRGB and supported CICP signals are honored. PNG gAMA without a stronger signal uses its declared power transfer function. Unrecognized chromaticities, HDR signaling, unsupported profiles, and high precision are refused for scoring and resizing.

ICC processing uses moxcms 0.8.1 (BSD-3-Clause OR Apache-2.0). Verified paths are RGB matrix profiles and gray profiles. Float transforms disable fixed-point preference and enable extended range. Out-of-sRGB-gamut results fail. Only numerical endpoint error within 0.00001 is rounded into the unit interval. Display P3 preparation is checked against independent LittleCMS float transforms on controlled in-gamut pixels. Saturated out-of-gamut P3 input is refused. Exact PNG paths retain 16-bit samples and color profiles without requiring scored conversion.

Preparation applies EXIF orientation once and keeps exact eight-bit alpha separately. Metric compositing belongs to the scoring operation. Hidden RGB remains in the encoded image. Prepared pixels are never published as flattened composites.

Resizing uses fast_image_resize with bilinear filtering in linear light. Alpha is premultiplied before filtering and unpremultiplied afterward. The result is converted back through the source color interpretation and retains the source profile. Dimensions preserve aspect ratio and never enlarge. Stored orientation and its tags remain consistent until stripping is requested.

Stripping removes EXIF, XMP and private PNG chunks while retaining required color signaling. It physically normalizes orientation before removing tags. Exact sample preservation refuses nonidentity orientation rather than pretending reordering is lossless. Tests cover all eight EXIF orientations and repeat preparation after encoding to catch double rotation.

The area limit remains 16,777,216 pixels. Float color buffers and metric pyramids increase the conservative working reservation to 256 bytes per pixel, up to 4 GiB per active image. This is an admission reservation, not a measured RSS result. The runtime must bound jobs and admission using that reservation. Compressed input and output remain capped at 128 MiB, and decoded private metadata at 8 MiB.

The legacy library's `ApplyICC::new` now accepts ICC bytes and returns a parse result. It no longer exposes a LittleCMS profile type. The operation supports RGB and RGBA integer samples at 8 or 16 bits. Other color layouts fail explicitly. This constructor change follows removal of the native color backend. LittleCMS remains a development-only independent reference.

Some nominal sRGB ICC files contain quantized colorants whose floating-point
transform leaves the destination gamut near its endpoints. These are refused
rather than clipped. The pinned astronaut photograph exercises this case:
independent LittleCMS also produces values outside the supported interval.
Exact sample-preserving paths can retain that profile. Its refusal is recorded
as incomplete work in benchmark results, never as a fast successful encode.
