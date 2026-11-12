# Retained behavior matrix

Oracle: `2708512`. Original codec benchmark: `66fc0fc`, with only its broken build script removed.
Generated fixture samples are defined by `tests/common/mod.rs::pixels`. Tests may run against the oracle with `RIMAGE_ORACLE_BIN`.

| Behavior | Disposition | Fixture and proof |
| --- | --- | --- |
| Exact RGB PNG dimensions and samples | retained | generated 256×192, `baseline::real_png_workload_has_independent_sample_proof`, independent `png` decoder |
| Unknown option exits 2 without writes | retained | generated PNG, baseline and `cli::command_help_invalid_combinations_and_removed_surface` |
| Backup collision preserves original | safety retained, backup CLI intentionally removed at S9 | baseline oracle branch; new destination-collision and alias regressions |
| Codec command aliases | intentionally removed at S9 | `cli::command_help_invalid_combinations_and_removed_surface` rejects retired commands |
| SVG | intentionally removed before campaign | CLI mixed input/output fixture verifies input refusal without publication |
| Traversal, globs, file lists, timestamps | retained; new output naming intentional | `deadlock`, `timestamps`, CLI ordered traversal and output-subtree exclusion; explicit file lists are relative to list |
| Metadata, color, alpha, precision | retained or explicitly refused | codec suites and `color_fidelity`; supported representations in `docs/COLOR.md` and `docs/AVIF.md` |
| JPEG progressive, gray, ICC/EXIF, corruption | retained | `jpeg_codec`; independent zune-jpeg decode, stored EXIF orientation |
| JPEG alpha flattening and high-precision reduction | intentionally refused | `jpeg_codec::invalid_quality_alpha_precision_and_corruption_are_named_errors` |
| WebP exact samples and hidden RGB, both alpha modes | retained | `webp_codec`, independent libwebp decode |
| PNG 8/16-bit gray/RGB/alpha and expanded palette samples | retained | `png_codec`, independent png 0.18 decode |
| Palette quantization | explicitly lossy | `png_codec::direct_palette_mode_is_explicit_and_keeps_dimensions` |
| AVIF opaque eight-bit SDR | retained with corrected color handling | `avif_codec`, independent dav1d decode at direct quality 0, 80, 100 |
| AVIF alpha, monochrome, HDR/high depth, grid, animation, container transforms | intentionally refused until validated | `avif_codec`; failing alpha experiment preserved at `a4d2fb5`, details in `docs/AVIF.md` |
| Exit aggregation and resource limits | retained with mandatory limits | `exit`, `codec_boundary`, CLI mixed zero-success and partial-success JSON; worker memory admission |
| Streams, signals, concurrent publication | retained and strengthened | `codec_boundary`, `signals`, CLI binary streams and broken-pipe tests |

The baseline harness selects legacy argv only for explicitly selected historical binaries. Production accepts only the new operation-first surface. Removed resize syntax and arbitrary preprocessing flags are intentionally replaced by explicit max-dimension. No retained behavior remains open.
