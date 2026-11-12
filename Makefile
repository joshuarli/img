.PHONY: test baseline-check decoder-reference-check

export RUSTC_WRAPPER := kache
export TMPDIR := $(CURDIR)/.work/tmp

test:
	cargo test --locked --all-features

baseline-check:
	cargo test --locked --all-features --test baseline

decoder-reference-check:
	cargo test --locked --all-features --test jpeg_codec --test webp_codec --test png_codec --test avif_codec --test codec_boundary
