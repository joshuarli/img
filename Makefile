.PHONY: test baseline-check decoder-reference-check ci release verify-release

TARGET ?= $(shell rustc -vV | awk '/^host:/ {print $$2}')

export RUSTC_WRAPPER := kache
export TMPDIR := $(CURDIR)/.work/tmp

test:
	@mkdir -p .work/tmp
	cargo test --locked --all-features

ci:
	./scripts/ci-check

release:
	@mkdir -p .work/tmp
	RUSTFLAGS="$(if $(findstring -linux-musl,$(TARGET)),-C link-arg=-fuse-ld=mold -C target-feature=+crt-static,$(RUSTFLAGS))" cargo build --locked --profile dist --features build-binary --target $(TARGET)
	$(MAKE) verify-release TARGET=$(TARGET)

verify-release:
	python3 scripts/verify-release.py target/$(TARGET)/dist/rimage $(TARGET)

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

pure-rust-build-check:
	./scripts/check-pure-rust-build
