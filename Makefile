.PHONY: test baseline-check decoder-reference-check

export RUSTC_WRAPPER := kache
export TMPDIR := $(CURDIR)/.work/tmp

test:
	cargo test --locked --all-features

baseline-check:
	cargo test --locked --all-features --test baseline

decoder-reference-check:
	cargo test --locked --all-features --test jpeg_codec --test webp_codec --test png_codec --test avif_codec --test codec_boundary

metric-reference-check:
	cargo test --locked --all-features --test metric_reference -- --ignored --nocapture

benchmark-verify:
	cargo test --locked --all-features --test benchmark_verify

benchmark-run:
	perf-gate-ready
	cargo build --locked --profile dist --features build-binary
	cargo run --locked --profile dist --all-features --example benchmark -- run
	perf-gate-ready -1
