# Test measurements

Run a focused Rust test target for an edit. `make test` runs the all-features suite and benchmark harness tests; reference checks are opt-in.

The JPEG marker walker searches for the next `0xFF` after a stuffed zero or restart marker. Leaving that non-`0xFF` marker byte unskipped reaches the same next marker.

WebP output has an independent libwebp decode check in `tests/webp_codec.rs`. The webpkit conformance test compares decoded RGBA bytes against the recorded libwebp reference.
