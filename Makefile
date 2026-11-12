.PHONY: test baseline-check

export RUSTC_WRAPPER := kache
export TMPDIR := $(CURDIR)/.work/tmp

test:
	cargo test --locked --all-features

baseline-check:
	cargo test --locked --all-features --test baseline
