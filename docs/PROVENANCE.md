# Source and fixture provenance

The retained-behavior oracle is `2708512`. The original codec benchmark is `66fc0fc`.
The upstream source identifies Vladyslav Vladinov as author and states `MIT OR Apache-2.0` licensing.
Original notices are preserved in `LICENSE-MIT` and `LICENSE-APACHE`, retrieved on 2026-11-11 from:

- https://github.com/SalOne22/rimage/blob/master/LICENSE-MIT
- https://github.com/SalOne22/rimage/blob/master/LICENSE-APACHE
- https://github.com/SalOne22/rimage/blob/master/Cargo.toml

Upstream HEAD at retrieval: `978a87075ad60fbbd033ba851036fbe84e9e0f47`.
SHA-256: MIT `cd5681828f9fe1cd224e817fb50be47d0fa16d6414a15c5f530910c276a8e50d`, Apache `4ec8872ad1b5ac7de77fc46235e2a4a97201c5c55426f2635764b0f5c56decef`.

This fork modifies the upstream source. Campaign additions follow the selected AGPL-3.0 distribution path when Imazen dependencies are integrated.
Upstream notices remain applicable to the original material. The resolved production audit is recorded below.

`tests/common/mod.rs` creates procedural RGB samples for the campaign. These generated samples are offered under CC0-1.0.
Existing `tests/files` arrived in the source history without per-file photographic provenance. They remain regression inputs and do not establish a redistributable benchmark corpus.
Do not distribute them as a newly licensed photographic corpus without establishing their origin.

## Resolved production audit

[production-dependencies.json](../benchmarks/production-dependencies.json) records 137 packages in the union of normal/build edges for default plus build-binary on macOS ARM64 and Linux musl ARM64/x86-64. It pins declared and selected licenses, repository/source and build-script presence. Cargo.lock owns exact versions/checksums. Development edges are excluded; the root package and build-only helpers are included.

| Component | Resolved version | Selected license |
| --- | --- | --- |
| zenjpeg / zenwebp / zenpng | 0.8.4 / 0.4.6 / 0.1.4 | AGPL-3.0-only |
| zenavif / zenravif / zenrav1e | 0.1.6 / 0.1.3 / 0.1.4 | AGPL-3.0-only |
| rav1d-safe / zenflate / zenquant | 0.5.7 / 0.3.6 / 0.1.3 | AGPL-3.0-only |
| zenavif-parse / zenavif-serialize | 0.6.2 / 0.1.4 | MPL-2.0 / BSD-3-Clause |
| fast-ssim2 | 0.9.1 | BSD-2-Clause AND BSD-3-Clause AND MIT |
| moxcms / png / libm | 0.8.1 / 0.18.1 / 0.2.16 | Apache-2.0 / MIT / MIT |
| retained imagequant operation | 4.4.1 | GPL-3.0-or-later |

The combined work follows AGPL-3.0-only; dependency files keep their own licenses and notices. This does not relabel MPL, GPL or permissive dependency source as AGPL. GPLv3 section 13 permits combination with AGPLv3 under its stated conditions. MPL 2.0 section 3.3 permits a larger work while retaining its covered-source obligations. Preserve notices and supply corresponding source under the applicable terms when distributing; no commercial grant or publication is assumed. Primary terms: [AGPL text](../LICENSE-AGPL3), [MPL 2.0](https://www.mozilla.org/en-US/MPL/2.0/). zenrav1e's [upstream PATENTS](../licenses/PATENTS-zenrav1e) remains applicable; the [MPL text](../licenses/MPL-2.0) is preserved locally.

Permissive alternatives are selected in the inventory; self_cell chooses Apache-2.0, avoiding its GPL-2.0-only alternative. unicode-ident also requires Unicode-3.0. No package has an unknown declared license. Published locked releases are used without upstream workspace patches, vendoring or Git dependencies. The actively maintained Imazen stack supplies the required pure-Rust codecs; adapters and removal conditions are documented at [the boundary](CODEC-BOUNDARY.md). The png adapter is required for precision/profile correctness. libm was already resolved and now supplies platform-stable transfer math. Their required artifact/RSS costs are reported in [benchmarks](../BENCHMARKS.md).

`png` 0.18.1 also independently verifies test samples. `rustybench` 0.5.0 (MIT) is development-only. Native dav1d/libwebp/LittleCMS references remain development-only and do not enter normal binary linkage. Source packages and upstream legal notices remain available through the locked registry sources; this local campaign does not create a distributable release bundle.
