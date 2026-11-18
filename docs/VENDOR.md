# Vendored upstream sources

Production codec dependencies that are vendored live under `vendor/<crate>` and
are wired in through `[patch.crates-io]` in `Cargo.toml`, so the lockfile names
the exact snapshot. Every snapshot is an unmodified copy of a published crate
unless its entry lists local patches. `scripts/check-native-build` compiles the
release binary and fails if any native source outside an allowlisted snapshot
is compiled.

## oxipng 10.2.1

- Role: exact PNG optimization and encoding (`src/still/png.rs`).
- Upstream: <https://github.com/oxipng/oxipng>, commit
  `36f3ef8aac65ecf1761739ea2f2e530281f1312b` (crates.io `oxipng 10.2.1`,
  checksum `52a88e3e5c2ad82e691b219e2b45a90eb5a51d26a434c7e191563e957a48e07a`).
- License: MIT (`vendor/oxipng/LICENSE`).
- Features: `default-features = false`, `parallel`. The CLI (`binary`),
  `zopfli`, `sanity-checks`, `freestanding` and `system-libdeflate` are off.
- Local patches: none.

## libdeflater 1.26.1 and libdeflate-sys 1.26.1

- Role: Rust bindings (libdeflater) and bundled C build (libdeflate-sys) for
  libdeflate, the DEFLATE compressor OxiPNG uses.
- Upstream: <https://github.com/libdeflater/libdeflater>, commit
  `260e1e9a333e32e769855c1ad1f203289e0a3c8d` (crates.io checksums
  `0cbb8285d2e2bd4b8d4155840ae81929354b4ba0d7b3c272c84761ffa2141e1f` and
  `d7870e5fbd2766179a937c725fb11f4ca0ef025d982beb61bd3ce755425bd19c`).
- Bundled C: libdeflate v1.26 from <https://github.com/ebiggers/libdeflate>.
  `vendor/libdeflate-sys/libdeflate/` is byte-identical to the v1.26 release
  tarball (sha256
  `bba03fffc5538576213675ce6968fcff6ce2e67d82e4d5febea2d05f9f13cf85`).
- Licenses: libdeflater/libdeflate-sys Apache-2.0
  (`vendor/libdeflater/LICENSE`, `vendor/libdeflate-sys/LICENSE`);
  libdeflate MIT (`vendor/libdeflate-sys/libdeflate/COPYING`).
- Build: the crate's `build.rs` compiles the eleven libdeflate library
  sources statically with `cc`, including libdeflate's ARM and x86 runtime
  CPU-feature dispatch. The `dynamic` feature is off, so no system libdeflate
  is probed or linked.
- Supported targets: aarch64-apple-darwin, aarch64-unknown-linux-musl,
  x86_64-unknown-linux-musl (static).
- Local patches: none.

## Updating a snapshot

1. Bump the version in `Cargo.toml` and run `cargo update -p <crate>` with the
   patch entry temporarily removed, so Cargo records the new registry checksum.
2. Replace `vendor/<crate>` with the extracted crate
   (`~/.cargo/registry/src/*/<crate>-<version>`) and confirm with
   `diff -r` that it is byte-identical.
3. For libdeflate-sys, diff `libdeflate/` against the upstream release tarball
   and record its sha256.
4. Restore the patch entry, update this file, and rerun
   `make native-build-check`, the PNG codec tests and the PNG benchmark.

Binary distributions carry the license files listed above; see
`docs/PROVENANCE.md`.
