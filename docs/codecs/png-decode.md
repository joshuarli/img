# PNG decode exactness investigation

The qualification stopped before timing because zune-png 0.5.2 returned incorrect alpha samples for valid 16-bit tRNS inputs. png 0.18.1 with EXPAND supplies the reference in native byte order, matching `src/still/png.rs`. Zen is an AGPL benchmark-only dependency; it is not integrated. Zen's RGBA output for grayscale with transparency is reduced to gray and alpha before comparison.

On macOS ARM64, 112 inputs (89 existing corpus and dev PNGs, 23 generated edge cases) produced two zune-png mismatches and zero zenpng mismatches. In `gray16-trns.png`, transparent value 256 instead makes sample 1 transparent; in `rgb16-trns.png`, the declared transparent RGB value remains opaque. The native-byte-order sample dumps are emitted by the comparison program.

Reproduce from the repository root with the existing dev fixture directory:

```sh
python3 benchmarks/codecs/png/decode-exact-fixtures.py
benchmarks/codecs/png/decode-exact-build.sh
.work/tmp/png-exact/target/release/png-exact .work/tmp/png-exact/fixtures/*.png /Users/josh/d/img/.work/tmp/pngdev/*.png benchmarks/corpus/*.png benchmarks/corpus/generated/*.png
```

The comparison exits 1 for the two differences. No timed qualification, corrupt-input qualification, C decoder evaluation, or integration was performed after the specified stop condition.
