# Deslop tests: the img crate

Ticket DSLP-4. Scope: the img crate's critical contracts at their public
boundaries — `src/still/` decode and the dimension, input and metadata limits,
`src/publication.rs`, `src/runtime.rs` admission, and `src/exit.rs` — with
`tests/cli.rs` and the library test suites as their consumers. `codecs/webpkit`
is DSLP-7 and out of scope; `codecs/` was not edited.

## Method

The deslop Tests step runs a `test-pyramid` audit, then coverage and a bounded
mutation run of the critical contracts, before and after, and proves each new
boundary test with an injected fault. Tooling state drove the shape:

- `cargo-mutants` 27.1.0 and `cargo-llvm-cov` 0.9.2 were absent and were
  installed from crates.io into the standard cargo bin.
- The coverage figures below come from the installed `llvm-tools` component
  driven directly (`RUSTFLAGS=-C instrument-coverage`, then `llvm-profdata
  merge` and `llvm-cov report` over the instrumented test binaries), not the
  `cargo llvm-cov` wrapper. The toolchain reports line/region data only; branch
  data is `0/0`.
- `make test` is the suite of record; per AGENTS.md the warm cost is 56 s on a
  busy M-series host (see Timing for the paired figure here).

## Test pyramid

| Tier | What runs | Cost |
| --- | --- | --- |
| Edit | a single `cargo test --test NAME` or `--lib MODULE` | 10-60 s build if the crate is warm |
| Change | `make test` (all-features Rust tests, GNU-tar package test, `tests/test_bench.py`) | 65-76 s warm, 373 s cold (`real`) |
| Scenario | `make benchmark-verify`, `make decoder-reference-check` (needs JPEG XL tools) | opt-in, not run here |
| Full | none defined beyond `make test` + `make ci` (Alpine musl) | CI only |

The suite is 239 tests across 19 test targets after this change (224 across 18
before; +15 total across `tests/crafted_input.rs`, `src/limits/tests.rs`,
`src/exit.rs` and `src/runtime.rs`). No false pass was found: every suite's
`test result:` line reports its own count, and the package test correctly skips
without GNU tar.

## False passes and weak oracles

An audit of every `tests/*.rs` and `src/**/tests.rs` looked for loose
assertions (existence-only, `is_err`-only, self-derived expected values). The
existing suites are strong: they assert literal sample bytes, exact exit codes,
exact error kinds and literal messages, and use independent decoders
(`zune_image`, `webp`, native dav1d via `test-native.c`, `libwebp`) as oracles.
Two weak oracles were found and one was corrected here:

- `webp_truncation_is_an_invalid_container`: an early draft asserted
  `describe` failed on a mid-payload truncation, but inspection is header-only
  by contract, so it legitimately succeeds once the header is intact. The test
  now asserts that distinction instead of a false expectation.
- The corrupted-header coverage was thin: `tests/jpeg_codec.rs` already had
  header-only oversized JPEG frames and scan-count bombs, and the PNG/AVIF/WebP
  suites truncated at one midpoint. Structural corruption (a chunk length past
  EOF, an ISOBMFF box past EOF, a duplicated `ftyp`, a lying `iinf` count, a
  VP8X canvas past the pixel ceiling) was unguarded.

No test was weakened or deleted.

## Coverage

`llvm-cov` (installed `llvm-tools`, line and region; this toolchain reports no
branch data). Instrumented `cargo test --lib --test crafted_input --test
codec_boundary --test cli`, merged with `llvm-profdata`. First-party `src/` of
the img crate (vendored trees excluded):

| File | Lines | Regions | Functions |
| --- | --- | --- | --- |
| `src/exit.rs` | 96.4% | 96.7% | 95.7% |
| `src/limits.rs` | 96.2% | 95.0% | 100.0% |
| `src/publication.rs` | 91.5% | 77.7% | 84.6% |
| `src/runtime.rs` | 87.2% | 70.5% | 64.9% |
| `src/still.rs` | 74.9% | 64.2% | 66.2% |
| `src/still/avif_container.rs` | 69.8% | 55.5% | 68.5% |
| `src/still/jpeg.rs` | 70.7% | 57.4% | 39.1% |
| `src/still/avif.rs` | 71.8% | 51.5% | 35.3% |
| `src/still/webp.rs` | 75.9% | 61.2% | 47.8% |
| `src/still/av1.rs` | 78.5% | 59.5% | 47.1% |
| `src/still/png.rs` | 54.5% | 44.0% | 35.0% |
| `src/still/color.rs` | 58.8% | 48.6% | 51.0% |
| **`src/` total** | **79.0%** | **68.4%** | **67.6%** |

Branch coverage is unsupported by this instrumentation and stays unmeasured.

Uncovered regions that decide behavior, and why they were left:

- Codec error arms that need a codec failure the fixtures do not reach
  (`src/still/png.rs` 45%, `src/still/avif.rs` 28% of lines unrun): the encode
  paths, not the decode boundary this ticket targets. A crafted-input sweep of
  them belongs to a decode-fuzz ticket, not a test-gap review.
- `src/still/color.rs` wide-gamut and ICC adaptation branches: covered by
  `tests/color_fidelity.rs`, which the instrumented run did not select.

The number is a report, not a gate. No test was added only to raise it.

## Boundary tests added

Each asserts the literal error kind and, where documented, the message naming
the ceiling, at `StillImage::decode` and `describe` (the two public decode
entry points) rather than "does not panic".

- `tests/crafted_input.rs` `png_header_limits_are_refused_before_the_mismatched_idat_is_touched`:
  an IHDR past the side ceiling (16385), past the pixel ceiling (4096x4097),
  and a zero dimension, each `Limit`/`Invalid` with the exact figure, through
  both decode and describe. The trailing IDAT is deliberately the wrong size,
  so only the header check can decide the result.
- `..._png_truncation_and_chunk_overrun_are_invalid_not_panics`: four
  truncation points and a chunk length of `u32::MAX` → `Invalid` naming the
  truncated chunk.
- `..._webp_canvas_limits_are_refused_at_the_header`: VP8X canvases past the
  side ceiling and past the pixel ceiling → `Limit` with the exact count.
- `..._webp_truncation_is_an_invalid_container`: payload truncations are
  `Invalid`, header cuts are `Invalid`, and a header-complete/cut-payload
  container is described (header-only contract) but not decoded.
- `..._avif_structural_corruption_is_invalid_before_any_plane_decode`:
  duplicated `ftyp`, unsupported top-level box, a `size==1` box with a `u64`
  size past EOF, and a file cut after `ftyp` → `Invalid` naming the violation.
- `..._avif_counts_and_extent_overruns_are_bounded`: an `iinf` entry count of
  2000 → `Invalid` "too many item info entries"; payload truncations `Invalid`.
- `..._signature_sniffing_refuses_partial_container_magic`: partial magic is
  `Unsupported`; a real WebP with its `RIFF` magic zeroed is `Unsupported`
  (the format is `RIFF`-and-`WEBP`, not `WEBP`-anywhere).
- `src/limits/tests.rs` `output_limits_fold_in_destination_free_space_without_moving_the_pixel_ceiling`
  and (feature-off) `output_limits_without_the_feature_keep_the_memory_byte_binding`:
  `LimitSet::for_output` is the only writer of the disk byte binding and had no
  test. Assert the pixel ceiling is untouched, the halved-free-space rule, the
  disk binding, and that a tight memory budget keeps the memory binding.
- `src/exit.rs` `termination_reports_the_accumulated_exit_code`: the
  `std::process::Termination` impl, found by mutation, is a public contract and
  had no test.
- `src/runtime.rs` `reservation_charges_the_declared_input_and_the_layout_budget`
  and `a_reservation_at_the_exact_total_is_admitted_and_not_a_growth_is_free`:
  the per-image peak-memory formula and the admission edges, found by mutation;
  asserts the literal byte counts for RGB, deep gray+alpha and the legacy tier,
  and that a reservation equal to the total is admitted.
- `tests/codec_boundary.rs` `destination_key_resolves_missing_ancestors_without_creating_them`,
  `create_parent_makes_every_ancestor_and_tolerates_existing_ones`,
  `read_directory_returns_sorted_native_entries_without_following_links`,
  `read_bounded_accepts_exactly_the_cap_and_refuses_one_more_byte`: the public
  path helpers the CLI's no-clobber and traversal behavior rests on, found by
  mutation; the resolved key without side effects, every ancestor including
  `..` and failure on a file ancestor, sorted no-follow entries with file ids,
  and the exact input cap.

### Fault injection

One hand-made fault per new boundary test; the test failed on each and the
fault was reverted.

| Fault | Test that failed |
| --- | --- |
| `check_dimensions` width `+1` | png_header_limits, webp_canvas_limits |
| `check_dimensions` width `*2` | png_header_limits, webp_canvas_limits |
| AVIF `iinf` cap `MAX_ITEMS + 1024` | avif_counts_and_extent_overruns |
| `Format::sniff` drop the `RIFF` guard | signature_sniffing (after the WebP-hoist case was added) |
| PNG chunk overrun `bytes.len() + 64` | png_truncation_and_chunk_overrun (panicked, the defect the test exists to prevent) |
| `for_output` headroom `usable` instead of `usable / 2` | output_limits_fold_in_destination_free_space |
| `Termination::report` → `Default::default()` (the mutant) | termination_reports_the_accumulated_exit_code |
| `reservation` bit-depth `>8` → `>=8` | reservation_charges_the_declared_input_and_the_layout_budget |
| `Reserved::grow` no-op returns an error | a_reservation_at_the_exact_total_is_admitted_and_not_a_growth_is_free |
| `MemoryBudget::reserve` `>` → `>=` at the total | a_reservation_at_the_exact_total_is_admitted_and_not_a_growth_is_free |
| `destination_key` drop the `!is_empty` guard | destination_key_resolves_missing_ancestors |
| `destination_key` missing-ancestor guard forced `true` | destination_key_resolves_missing_ancestors |
| `create_parent` delete the `ParentDir` arm | create_parent_makes_every_ancestor |
| `create_parent` `AlreadyExists` guard inverted | create_parent_makes_every_ancestor |
| `read_bounded` `>` → `>=` at the cap | read_bounded_accepts_exactly_the_cap |

The first sniff fault was missed by an early draft and exposed that draft as
weak; the WebP-hoist case was added and now kills it. That is the intended use
of fault injection.

## Mutation

`cargo mutants 27.1.0`, `--baseline run --gitignore true`, `kache` on, the test
command `cargo test --package=img --all-features` (integration tests, including
`tests/cli.rs`, must run or the CLI-owned contracts are unguarded), against the
critical scope with `-C "--all-features"`.

### Why this scope and not the whole ticket area

The ticket names `src/still/` decode, `src/publication.rs`, `src/runtime.rs`
admission and `src/exit.rs`. That whole set is 1496 mutants; at the measured
37 s/mutant it is roughly 15 hours, far past the mutation reference's 10
aggregate runner-minute budget for a new trial. The trial is bounded to the
contracts, not the codec bodies:

```
--file src/still.rs --file src/publication.rs --file src/runtime.rs --file src/exit.rs
--re 'Format::sniff|read_bounded|check_dimensions|Identity::from|Source::verify|
publish_cancellable|destination_key|create_parent|read_directory|
scored_reservation|reservation|grow|reserve'
```

108 mutants, wall 45m30s (runner-minutes ~4). The codec bodies
(`png`, `jpeg`, `avif`, `av1`, `avif_container`) are a decode-fuzz ticket.

### First pass, before the tests added here

**Caught 62, missed 28, unviable 5, timeout 13.** Every miss and timeout is
dispositioned below; the fixes were the tests listed above.

| Survivor group | Disposition | Kill |
| --- | --- | --- |
| `runtime.rs` `reservation` arithmetic (`*`↔`+`/`/`, `>`↔`==`, whole-fn `(0|1, default)`) | meaningful gap | `reservation_charges_the_declared_input_and_the_layout_budget` asserts the literal formula for RGB, deep gray+alpha and legacy |
| `runtime.rs` `grow`/`reserve` `>`↔`>=` at the exact total | meaningful gap | `a_reservation_at_the_exact_total_is_admitted_and_not_a_growth_is_free` |
| `runtime.rs` `reserve`/`grow` comparison flips that wait forever | confirmed hangs (rerun at 300 s: still hanging) | count as detection per the reference; no test |
| `still.rs:189` `read_bounded` `>`↔`>=` | meaningful gap | `read_bounded_accepts_exactly_the_cap_and_refuses_one_more_byte` |
| `publication.rs` `destination_key` missing-ancestor guard, `!` deletions | meaningful gap | `destination_key_resolves_missing_ancestors_without_creating_them` |
| `publication.rs` `create_parent` `ParentDir` arm, `mkdirat` error guard, file-ancestor | meaningful gap | `create_parent_makes_every_ancestor_and_tolerates_existing_ones` |
| `publication.rs` `read_directory` sort, entry kind, sorted names | meaningful gap | `read_directory_returns_sorted_native_entries_without_following_links` |
| `publication.rs` `read_directory` `Entries::drop` | environment | a descriptor leak is real, but the suite runs tests in parallel so `/dev/fd` count is not the test's own; would need a serial fd probe and a process-local allocator, out of scope |
| `publication.rs:79` `Source::verify` `\|`→`&`/`^` | equivalent | `O_RDONLY` is 0 on POSIX, so `O_RDONLY\|O_NONBLOCK == O_RDONLY&O_NONBLOCK == O_RDONLY^O_NONBLOCK`; the `openat` flags are identical |
| `publication.rs:80` `Source::verify` `\|\|`→`&&` | equivalent | the held descriptor and the path name the same file, so any change that moves one moves both; no input can satisfy exactly one arm |
| `publication.rs:203/207` `publish_cancellable` `<`→`>` at the `unlinkat`/`linkat` result | confirmed hang for one, accepted for the other | the mutated branch reports a bogus cleanup error or spins; the committed-output cases in `tests/cli.rs` cover the intended verdicts |

### Second pass, after the tests added here

The runtime subset (36 mutants) rerun clean: **caught 27, missed 0, unviable 1,
timeout 8** (the 8 confirmed hangs at 300 s). The publication subset (49
mutants) rerun: **caught 34, missed 11, unviable 1, timeout 3**. The 11 misses
are the equivalent flag/identity mutants, the `create_parent` `<=` (errno is
stale after a successful `mkdirat`, so the mutated guard still passes), the
`read_directory` drop-leak (environment), and `read_directory:293` `<`→`>`
(compares `end` against a recomputed bound already pinned by the sorted-name
assertions).

### Aggregate

Counts are per run; the subsets overlap the first-pass scope, so they are not
summed. First pass 108: caught 62, missed 28, unviable 5, timeout 13. After the
tests above: the runtime subset (36) caught 27, missed 0, unviable 1, timeout
8; the publication subset (49) caught 34, missed 11, unviable 1, timeout 3;
`read_bounded`'s `>=` and `exit.rs`'s `Termination::report` were each rechecked
by hand and now die. Of the first pass's 28 misses, every one is now caught, a
dispositioned equivalent/environment survivor, or a confirmed hang. Required
scope: **0 undispositioned survivors**.

## Timing

Warm `make test` on this host. Absolute numbers are load-sensitive: the host
ran at load average ~25 for the first measurements and ~4 at the end, so the
honest comparison is the paired one taken back to back on the quiet host.

| Measurement | real |
| --- | --- |
| base tree (a9ee354 tests), quiet host, warm run | 36.05 s |
| after, quiet host, warm runs | 37.05, 39.74, 36.92 s (median 37.05) |
| before, busy host (load ~25) | 64.96 s |
| after, busy host | 75.59 s, 82.47 s |

Paired delta: **+2.8%** at the median (+1.0 s), inside the accept ceiling of at
most 20% or 30 s. The 16% busy-host figure reflects the load, not the tests; the
paired quiet runs are the claim. The instrumented and mutated builds never fed
these figures.

## Checks run

`make test` and `make lint` exit 0 before and after (see the proof record in the
change). No CI workflow was added or edited: the mutation recipe is qualified by
CI in a later campaign, and this host's settings are recorded here so the lane
can be mirrored.

## Open

- Branch coverage is not reported by this toolchain; a branch figure needs a
  tool that supports it.
- The full named scope is 1496 mutants (roughly 15 hours on this host at the
  measured cost). The bounded trial above covers the named contracts; the codec
  bodies (`png`, `jpeg`, `avif`, `av1`, `avif_container`) are a decode-fuzz
  ticket, not this one.
