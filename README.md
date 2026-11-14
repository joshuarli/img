# Optimize and convert still images

Rimage optimizes JPEG, PNG, WebP and AVIF with verified decoded fidelity.
New outputs use atomic publication and refuse existing destinations.
This branch contains an operation-first CLI with breaking changes from 0.14.0.

In `~/usr`, run `sh sync tools rimage` with `gh` authenticated for this private
repository. Prereleases provide checksummed macOS ARM64 and static Linux
ARM64/x86-64 binaries. See [installation and release verification](docs/RELEASE.md).

Optimize a file at the default minimum SSIMULACRA2 score of 95:

```sh
rimage optimize photo.jpg
```

The output is `photo.optimized.jpg`. If no strictly smaller qualifying candidate
exists, Rimage copies compliant source bytes and reports `unchanged`.
To authorize replacement of the source, add `--in-place`. Keep external editors
and other writers away from that file until Rimage finishes.

Choose among compatible formats at a requested score:

```sh
rimage optimize photo.jpg --format auto --score 90 --effort balanced
```

Convert to an inferred destination format:

```sh
rimage convert photo.png -o photo.webp
rimage convert photo.jpg --max-dimension 2048 -o resized.webp
rimage convert diagram.png --lossless -o diagram.webp
```

Inspect headers or compare actual encoded representations:

```sh
rimage inspect photo.jpg --json
rimage compare photo.png photo.webp --json
```

Process a directory with bounded image workers:

```sh
rimage optimize ./images --recursive --out-dir ./out --jobs 8
```

Stream one image through a concrete format:

```sh
cat input.png | rimage convert - --format webp --lossless -o - > output.webp
```

Run `rimage --help` for flags. See [CLI contracts and JSON](docs/CLI.md),
[optimization](docs/OPTIMIZATION.md), [color and metadata](docs/COLOR.md),
[AVIF limitations](docs/AVIF.md), and [independent scoring](docs/SCORING.md).
[Benchmark evidence](BENCHMARKS.md) distinguishes measured results from deferred gates.

Supported hosts are macOS ARM64 and Linux ARM64/x86-64. For source builds and
contributing, see [development checks](docs/CODEC-BOUNDARY.md#development-checks).
Open defects are tracked in [TODO.md](TODO.md).

Rimage's source is licensed under **Apache-2.0**; the upstream MIT notice is
preserved. Built binaries link Imazen codecs licensed AGPL-3.0-only and
imagequant licensed GPL-3.0-or-later, so binary distributions follow
**AGPL-3.0-only**. See [license provenance](docs/PROVENANCE.md) and the
license files. No commercial codec license is assumed.
