# Retained behavior matrix

Oracle: `2708512`. Original codec benchmark: `66fc0fc`, with only its broken build script removed.
Generated fixture samples are defined by `tests/common/mod.rs::pixels`. Tests may run against the oracle with `RIMAGE_ORACLE_BIN`.

| Behavior | Disposition | Fixture and proof |
| --- | --- | --- |
| Exact RGB PNG dimensions and samples | retained | generated 256×192, `baseline::real_png_workload_has_independent_sample_proof`, independent `png` decoder |
| Unknown option exits 2 without writes | retained | generated PNG, `baseline::invalid_parser_and_backup_collision_preserve_inputs` |
| Backup collision preserves original | retained safety, backup CLI intentionally removed at S9 | same pilot and `deadlock` backup regressions |
| Codec command aliases | intentionally replaced at S9 | existing `cli` suite until S9 |
| SVG | intentionally removed before campaign | `cli::unsupported_vector_input_does_not_publish_output` |
| Traversal, globs, file lists, timestamps | open | existing `deadlock`, `timestamps`, path unit tests |
| Metadata, color, alpha, precision | retained or explicitly refused | codec suites and `color_fidelity`; supported representations in `docs/COLOR.md` and `docs/AVIF.md` |
| JPEG progressive, gray, ICC/EXIF, corruption | retained | `jpeg_codec`; independent zune-jpeg decode, stored EXIF orientation |
| JPEG alpha flattening and high-precision reduction | intentionally refused | `jpeg_codec::invalid_quality_alpha_precision_and_corruption_are_named_errors` |
| WebP exact samples and hidden RGB, both alpha modes | retained | `webp_codec`, independent libwebp decode |
| PNG 8/16-bit gray/RGB/alpha and expanded palette samples | retained | `png_codec`, independent png 0.18 decode |
| Palette quantization | explicitly lossy | `png_codec::direct_palette_mode_is_explicit_and_keeps_dimensions` |
| AVIF opaque eight-bit SDR | retained with corrected color handling | `avif_codec`, independent dav1d decode at direct quality 0, 80, 100 |
| AVIF alpha, monochrome, HDR/high depth, grid, animation, container transforms | intentionally refused until validated | `avif_codec`; failing alpha experiment preserved at `a4d2fb5`, details in `docs/AVIF.md` |
| Exit aggregation and resource limits | open | `exit`, `limits` suites; mandatory limits at S1 |
| Streams, signals, concurrent publication | open | S1 and S9 |
