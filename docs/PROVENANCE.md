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
Upstream notices remain applicable to the original material. The final combined license and resolved dependency notices are audited at S11.

`tests/common/mod.rs` creates procedural RGB samples for the campaign. These generated samples are offered under CC0-1.0.
Existing `tests/files` arrived in the source history without per-file photographic provenance. They remain regression inputs and do not establish a redistributable benchmark corpus.
Do not distribute them as a newly licensed photographic corpus without establishing their origin.

`png` 0.18 is an independent Rust PNG decoder for test-only sample verification (MIT OR Apache-2.0).
`rustybench` 0.5.0 is the prescribed benchmark harness (MIT), maintained at https://github.com/laputa-systems/rustybench.
Both are development dependencies and are excluded from the production graph. Their build costs remain separate from production build comparisons.
