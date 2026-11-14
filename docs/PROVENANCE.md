# Source and fixture provenance

The retained-behavior oracle is `2708512`. The original codec benchmark is `66fc0fc`.
The upstream source identifies Vladyslav Vladinov as author and states `MIT OR Apache-2.0` licensing.
Original notices are preserved in `LICENSE-MIT` and `LICENSE-APACHE`, retrieved on 2026-11-11 from:

- https://github.com/SalOne22/rimage/blob/master/LICENSE-MIT
- https://github.com/SalOne22/rimage/blob/master/LICENSE-APACHE
- https://github.com/SalOne22/rimage/blob/master/Cargo.toml

Upstream HEAD at retrieval: `978a87075ad60fbbd033ba851036fbe84e9e0f47`.
SHA-256: MIT `cd5681828f9fe1cd224e817fb50be47d0fa16d6414a15c5f530910c276a8e50d`, Apache `4ec8872ad1b5ac7de77fc46235e2a4a97201c5c56decef`.

This fork modifies the upstream source. Rimage's source, including the upstream material and its modifications, is licensed under Apache-2.0, selected from the upstream `MIT OR Apache-2.0`; the upstream MIT notice remains applicable to the original material.

Binaries built from this tree link only permissively licensed production dependencies, so they carry no copyleft obligations. `LICENSE-AGPL3` applies to binaries published from before the permissive codec swap; it is retained for those distributions and for history, and binds nothing built today. [RELEASE.md](RELEASE.md) and `scripts/package-release.py` ship it as a notice for that reason.

`tests/common/mod.rs` creates procedural RGB samples. These generated samples are offered under CC0-1.0.
Existing `tests/files` arrived in the source history without per-file photographic provenance. They remain regression inputs and do not establish a redistributable benchmark corpus.
Do not distribute them as a newly licensed photographic corpus without establishing their origin.

## Resolved production audit

[production-dependencies.json](../benchmarks/production-dependencies.json) records the normal/build closure for default plus build-binary on macOS ARM64 and Linux musl ARM64/x86-64. It pins declared and selected licenses, repository/source and build-script presence. Cargo.lock owns exact versions/checksums. Development edges are excluded; the root package is included.

| Component | Resolved version | Selected license |
| --- | --- | --- |
| zune-jpeg (JPEG decode) | 0.5.15 | MIT OR Apache-2.0 |
| mozjpeg-rs (JPEG encode) | 0.9.2 | BSD-3-Clause |
| png (PNG) | 0.18.1 | MIT OR Apache-2.0 |
| quantette (palette quantization) | 0.6.0 | MIT OR Apache-2.0 |
| image-webp (WebP decode) | 0.2.4 | MIT OR Apache-2.0 |
| webpkit (WebP encode, vendored path dependency, experimental) | `cc1311ebf` | MIT OR Apache-2.0 |
| rav1d (AV1 decode) | 1.1.0 | BSD-2-Clause |
| ravif (AVIF encode) | 0.13.0 | BSD-3-Clause |
| zenavif-parse (AVIF container) | 0.6.2 | MPL-2.0 |
| fast-ssim2 | 0.9.1 | BSD-2-Clause AND BSD-3-Clause AND MIT |
| moxcms / libm / zune-image / zune-core | 0.8.1 / 0.2.16 / 0.5.0 / 0.5.1 | BSD-3-Clause OR Apache-2.0 / MIT / MIT OR Apache-2.0 OR Zlib / MIT OR Apache-2.0 OR Zlib |

No AGPL, GPL or LGPL package is in the default or build-binary graph; the LGPL entries in the lockfile graph are wasi-only disjunctive arms never selected on supported targets. zenavif-serialize 0.1.4 (BSD-3-Clause) is a development-only fixture tool. Native dav1d, libwebp and LittleCMS references remain development-only and never enter the production graph. The MPL-2.0 text is preserved locally ([licenses/MPL-2.0](../licenses/MPL-2.0)) as required by zenavif-parse. zenpixels and zencodec are Apache-2.0 OR MIT and are required as metadata/descriptor types shared with fast-ssim2 and ravif.

unicode-ident also requires Unicode-3.0. No package has an unknown declared license. Published locked releases are used without upstream workspace patches, and no Git dependencies are selected except the in-tree vendored path dependency documented below.

`png` 0.18.1 also independently verifies test samples. `rustybench` 0.5.0 (MIT) is development-only. Native dav1d/libwebp/LittleCMS references remain development-only and never enter the production graph. The [release source archive](RELEASE.md) contains the exact source tree, lockfile and vendored dependencies with their upstream notices.

## Permissive WebP candidate (experimental on this branch)

A source-only Webpkit snapshot (MIT OR Apache-2.0), with upstream authorship and license notices preserved, was imported under `codecs/webpkit/` from [P4suta/webpkit](https://github.com/P4suta/webpkit) at commit `cc1311ebf5a7b287b22f8ae2efb22aae90da49ef`. The imported source blobs were SHA-matched against the pinned upstream tree; the standalone manifest is locally reconstructed. This branch wires the snapshot in as a path dependency of the production WebP encoder through `src/still/webp.rs`, replacing zenwebp pending parity evidence from the [unattended campaign](WEBP-HILLCLIMB.md); master keeps zenwebp and the prior distribution statement until those gates pass and this branch is reviewed. See [WEBP-IMPORT.md](WEBP-IMPORT.md) and [WEBP-HILLCLIMB.md](WEBP-HILLCLIMB.md).
