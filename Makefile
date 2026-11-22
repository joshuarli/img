.PHONY: test lint baseline-check decoder-reference-check ci release verify-release bench bench-check bench-list

TARGET ?= $(shell rustc -vV | awk '/^host:/ {print $$2}')
CARGO_TARGET_DIR ?= target

export RUSTC_WRAPPER ?= $(shell command -v kache 2>/dev/null)
export TMPDIR := $(CURDIR)/.work/tmp

test:
	@mkdir -p .work/tmp
	cargo test --locked --all-features
	@if tar --version 2>/dev/null | grep -q 'GNU tar'; then python3 tests/test_package_release.py; else echo 'skipping tests/test_package_release.py: it needs GNU tar (release.yml runs it)'; fi
	python3 -m unittest -q tests.test_bench

lint:
	@mkdir -p .work/tmp
	cargo check --locked --no-default-features
	cargo check --locked --no-default-features --features build-binary
	cargo clippy --locked --all-features --all-targets

ci:
	./scripts/ci-check

release:
	@mkdir -p .work/tmp
	RUSTFLAGS="$(if $(findstring -linux-musl,$(TARGET)),-C link-arg=-fuse-ld=mold -C target-feature=+crt-static,$(RUSTFLAGS))" cargo build --locked --profile dist --features build-binary --target $(TARGET)
	$(MAKE) verify-release TARGET=$(TARGET)

verify-release:
	python3 scripts/verify-release.py "$(CARGO_TARGET_DIR)/$(TARGET)/dist/img" $(TARGET)

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

TIER ?= change
BENCH_ARGS ?=

bench:
	./scripts/bench run --tier $(TIER) $(BENCH_ARGS)

bench-check:
	./scripts/bench check --tier $(TIER) $(BENCH_ARGS)

bench-list:
	./scripts/bench list

native-build-check:
	./scripts/check-native-build

.PHONY: clean
clean:
	@if test -L ./target || { test -e ./target && ! test -d ./target; }; then \
	  printf '%s\n' 'make clean: ./target must be a local build directory' >&2; exit 1; \
	fi
	rm -rf -- ./target
