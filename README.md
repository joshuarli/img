# Optimize and convert still images

img optimizes JPEG, PNG, WebP and AVIF with verified decoded fidelity.
New outputs use atomic publication and refuse existing destinations.
This branch contains an operation-first CLI with breaking changes from 0.14.0.

In `~/usr`, run `sh sync tools img` with `gh` authenticated for this private
repository. Prereleases provide checksummed macOS ARM64 and static Linux
ARM64/x86-64 binaries. See [installation and release verification](docs/RELEASE.md).

Optimize a file at the default minimum SSIMULACRA2 score of 95:

```sh
img optimize photo.jpg
```

The output is `photo.optimized.jpg`. If no strictly smaller qualifying candidate
exists, img copies compliant source bytes and reports `unchanged`.
To authorize replacement of the source, add `--in-place`. Keep external editors
and other writers away from that file until img finishes.

Choose among compatible formats at a requested score:

```sh
img optimize photo.jpg --format auto --score 90 --effort balanced
```

Convert to an inferred destination format:

```sh
img convert photo.png -o photo.webp
img convert photo.jpg --max-dimension 2048 -o resized.webp
img convert diagram.png --lossless -o diagram.webp
```

Inspect headers or compare actual encoded representations:

```sh
img inspect photo.jpg --json
img compare photo.png photo.webp --json
```

Process a directory with bounded image workers:

```sh
img optimize ./images --recursive --out-dir ./out --jobs 8
```

Stream one image through a concrete format:

```sh
cat input.png | img convert - --format webp --lossless -o - > output.webp
```

Run `img --help` for the complete command reference and examples. See [CLI contracts and JSON](docs/CLI.md),
[optimization](docs/OPTIMIZATION.md), [color and metadata](docs/COLOR.md),
[AVIF limitations](docs/AVIF.md), and [independent scoring](docs/SCORING.md).
[Benchmark evidence](BENCHMARKS.md) distinguishes measured results from deferred gates.

Supported hosts are macOS ARM64 and Linux ARM64/x86-64. For source builds and
contributing, see [development checks](docs/CODEC-BOUNDARY.md#development-checks).
Open defects are tracked in [TODO.md](TODO.md).

img's source is licensed under **Apache-2.0**; the upstream MIT notice is
preserved. The production dependency closure contains no copyleft of any kind:
every production dependency is permissively licensed (MIT, Apache-2.0, Zlib,
BSD-2/3-Clause, ISC, Unicode-3.0, CC0-1.0 or a permissive dual license; the one
`license-file` entry, `quantizr`, is verbatim MIT). The AVIF container reader is
implemented in-tree. Binary distributions carry no AGPL, GPL or MPL obligations.
No commercial codec license is assumed. The WebP backend is experimental pending
the parity campaign below; see
[license provenance](docs/PROVENANCE.md) and the
license files.

## Permissive WebP replacement campaign

An isolated snapshot of the MIT/Apache-2.0 [Webpkit](https://github.com/P4suta/webpkit) codec lives in [`codecs/webpkit/`](codecs/webpkit/) and is wired in as the WebP encoder through a path dependency, replacing zenwebp. The executable unattended correctness/performance campaign is [`docs/WEBP-HILLCLIMB.md`](docs/WEBP-HILLCLIMB.md) and its exact import provenance is [`docs/WEBP-IMPORT.md`](docs/WEBP-IMPORT.md). The backend is experimental: the pinned snapshot shows photographic size regressions in [BENCHMARKS.md](BENCHMARKS.md), and the campaign gates govern any claim of parity.

The [permissive codec audit](docs/PERMISSIVE-CODEC-AUDIT.md) inventories the remaining AGPL/GPL roles and the leading pure-Rust alternatives for JPEG, PNG/DEFLATE, palette quantization and AVIF/AV1, with explicit integration blockers and performance validation gates.
