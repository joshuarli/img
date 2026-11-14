# Session handoff — WebP hillclimb (2026-11-14, post-rename)

Written at wind-down per owner's request. Read `docs/WEBP-HILLCLIMB.md` for the full campaign ledger and `BENCHMARKS.md` for the recorded matched-score tables. The destination gate (permissive WebP backend at `≤` withdrawn-zenwebp matched-score bytes, comparable or lower encode time) is **NOT met and is not claimed anywhere**.

## Identity note: the project was renamed upstream mid-session

`6c98669 refactor!: rename the project from rimage to img` (BREAKING CHANGE) is the current base. Package, lib, binary and CLI are now `img`; `RimageError` is now `ImgError`; `RIMAGE_*` environment variables are now `IMG_*` (`IMG_BENCH_TOOLS`, `IMG_BENCH_OUTPUT_PREFIX`, `IMG_RESULTS_NAME`, `IMG_BENCH_BIN`); release artifacts are prefixed `img-`; the repository is `joshuarli/img` (local checkout remains the directory `/Users/josh/d/img`). **Historical records, the original-build patch, and dated prose keep the old name** — in particular, the committed 2026-11-14 fresh benchmark records record the tool label `rimage` (measured with the pre-rename dist binary — byte-identical source) and the benchmark validator accepts both tool labels for retained records. New captures will record `img`.

## Repo state at handoff

- Master == origin/master == `940fb0d` (`docs(handoff): session handoff for the webp hillclimb`, rebased onto the rename).
- The tree contains no uncommitted codec, capture or result changes at the time of writing, apart from untracked scratch diagnostics created during this campaign (`examples/scratch-diff.rs`, `examples/scratch-lossless.rs`, `examples/scratch-score.rs`, `examples/scratch-sns.rs`) — mechanical, already migrated `rimage::` → `img::`, and compile-clean. Leave them or delete them at your discretion.
- Retained 2026-11-13 records (`benchmarks/results/untimed.json`, `timed.json`) untouched. Fresh 2026-11-14 captures committed: `untimed-2026-11-14.json` + `2026-11-14-*.webp` (matched-score WebP vs cwebp; `2139333`, `5f6f436`); `timed-2026-11-14.json` + `2026-11-14-timed-*.webp` (`573ca60`; quiet-host 5-rep timing; `timingGate: accepted`; revision `9542c98`). The fresh full-matrix attempt (`RIMAGE_BENCH_TOOLS=rimage,rimage-png,cwebp`, prefix `2026-11-14-full-`, `untimed-full-2026-11-14.json`, 98 records / 68 completed) was measured, confirmed refusal-identical to the retained records on all 30 non-completed cells, and **discarded** as byte-redundant with the committed captures.
- Verification: at `8c8eba5`/`8b5af0b` the codec suite was 911/911, the root crate green, `cargo test --locked --all-features --test benchmark_verify` green. The rename landed upstream without re-verification here; this session ran `cargo check --locked --all-features` (root) and `cargo check --locked --all-features --examples` — both green (build only; tests not re-run post-rename). Re-run **both** full suites plus `benchmark_verify` before trusting or landing anything on the renamed tree.

## Measured-but-unlanded experiment: deeper LZ77 chain (`MAX_CHAIN`)

`pub(crate) const MAX_CHAIN` in `codecs/webpkit/src/lossless/constants.rs` (hash-chain candidate cap for the VP8L LZ77 match finder in `codecs/webpkit/src/lossless/vp8l/backref.rs`; committed value 64). The staircase/DP candidate set scales with this cap, so it is a search axis the effort ladder never swept.

Direct-codec measurements (`examples/scratch-lossless.rs <png> <effort>`), bytes (encode wall):

| fixture | 64 (master) | 128 | 256 | 512 |
| --- | --- | --- | --- | --- |
| large.png level 1 | 12,682 (5.6 s) | 8,678 (10.7 s) | **4,854 (25.1 s)** | 4,668 (57.3 s) |
| large.png level 9 | 11,712 (23 s) | 8,118 (42.8 s) | **4,580 (99.4 s)** | 4,316 (238 s) |
| text level 1 | 72 | 58 | n/m | n/m |
| line level 1 | 64 | 62 | n/m | n/m |
| transparent level 9 | 196 | 130 | 128 | n/m |
| coffee level 9 | 386,802 | 353,636 | 353,636 | n/m |
| astronaut level 9 | ~347,890 | ~347,890 | 347,816 | n/m |
| grass level 9 | 212,590 | 212,400 | n/m | n/m |
| gravel level 9 | 188,642 | 188,456 | n/m | n/m |
| flat/gray/ui level 1 | 32/56/66 | 32/56/66 | 32/56/66 | n/m |

The withdrawn stack's `large.png` number is 7,014 B. Bytes are monotone non-increasing in the cap (the DP picks the cost-minimal candidate); every measured fixture improved or stayed flat. The 256 candidate **beats the withdrawn stack on `large.png` by −30.8% (level 1) and −34.7% (level 9)** and raises the lossless encode wall on large photographic inputs several-fold. No other corpus cell moves as decisively (coffee level 1 moves by −8 B; level 9 improves −33,166 B; astronaut/grass/gravel move a few hundred bytes).

Test fallout at 256 (full `cargo test --locked --lib`): 909/911 pass. The two failures are cap-pinned unit tests, not correctness bugs:

- `lossless::vp8l::backref::tests::find_stops_at_the_max_chain_hop_limit`
- `lossless::vp8l::backref::tests::find_candidates_stops_at_the_max_chain_hop_limit`

Both assert the walk stops exactly one hop short of a planted match at hash-chain depth 65 (behind 64 same-pair decoys). Re-derivation is mechanical: rebuild the fixture with decoy depth ≥ the chosen cap (e.g. 255/511 decoys for 256/512); the same invariant stays tested. Do **not** delete or weaken them.

Decision framework for the next session (the user authorized the whole remaining scope, verbatim "all authorized"):

1. **Recommended: land it tier-gated or globally at 128 or 256**, not 512 (diminishing returns vs wall). The matched-score cells use `Effort::Fast` (level 1) and the lossy ladder always emits the mandatory lossless candidate at that tier, so a global cap change also slows the lossy path noticeably — weigh that against the lossless bytes win. A tier-gated landing (64 at level 0–2, deeper at ≥3) preserves the production lossy wall while letting Best-effort lossless capture the win; a global landing chases the withdrawn `large.png` number outright. Either is defensible; both are explicitly documentable as a Pareto trade (bytes −30%+ vs wall ~2–4× on the lossless path only).
2. After landing: re-derive the two hop-limit tests; run both full suites (`cargo test --locked` at the root and inside `codecs/webpkit`) plus `cargo test --locked --all-features --test benchmark_verify`; then re-capture **all** fresh lossless cells (every `2026-11-14-*.webp` lossless cell and every lossless fallback) with the prefixed capture path (`IMG_BENCH_OUTPUT_PREFIX=2026-11-15- IMG_RESULTS_NAME=untimed-2026-11-15.json`) and rewrite `BENCHMARKS.md`; commit as one coherent `perf(vp8l): …` change; keep the 2026-11-13 retained records untouched regardless.
3. **Do not** re-pin or skip the two tests; do not accept the change without re-capturing the matched-score cells.
4. The encode-wall verdict (3.3–4.4× withdrawn; 3.6–5.7× cwebp quiet-host wall) is unaffected by this arm: VP8 lossy is a different path.

## Blocked items (user decisions / environment, not code)

- **Full production matrix re-score (630-cell)**: needs the pinned *original* toolchain at `.work/tmp/original-target/release/img` built from `66fc0fc` + `docs/original-build.patch` (mozjpeg 0.10.13, oxipng 10.2.1, ravif 0.13.0). It is absent in this checkout; the 2026-11-13 capture had it. The user's "all authorized" covers this rebuild. Once rebuilt: run the prefixed fresh capture with no tool filter (`IMG_BENCH_TOOLS` unset), e.g. `IMG_RESULTS_NAME=untimed-full-2026-11-15.json IMG_BENCH_OUTPUT_PREFIX=2026-11-15- cargo run --locked --profile dist --all-features --example benchmark -- capture`, commit the fresh records, and keep the retained records untouched. Note the retained and 2026-11-14 committed records carry the `rimage` tool label; fresh runs record `img` — the validator accepts both.
- **Linux x86-64 native timing**: requires a dedicated, non-shared x86-64 host. Emulated Docker amd64 is not claimable and must not be used for timing claims.
- **Lossy t95 RD-efficiency**: the remaining coding-layer gap after the lambda A/B was *rejected* (see `8c8eba5` and `8b5af0b`). Any further work there is new coefficient-modeling R&D, not a constant re-tune.
- **grass t80/t90 (+2.2%/+6.1%) vs cwebp** and **coffee t80 wall (1029 vs 288 ms)**: unchanged hypotheses of FDCT/trellis/coefficient modeling work; not driven further.

## State of play vs the destination gate

- Matched-score WebP (fresh 2026-11-14, committed): t80 wins vs cwebp on coffee (62,224 @ 80.6 vs 87,502 @ 85.8), gravel (49,966 @ 80.7 vs 59,728), large.png (12,682 lossless), all flat/gray/line/text/ui/compressed; loses grass t80 (+2.2%), transparent t80 (+16.7%), grass t90 (+6.1%), and falls back to lossless at t95 (+20–24% vs cwebp's lossy passes).
- Encode wall 3.3–4.4× the withdrawn closure; ~3.6–5.7× cwebp process wall on photographics (fresh timed capture, quiet host, `timed-2026-11-14.json`).
- VP8L `large.png` gap: +80.8% vs withdrawn today, with a measured exit (this document, `MAX_CHAIN`).
- WebP backend remains experimental; destination gate **not met**; no parity claim exists in the evidence. Any final report must state exactly that.
