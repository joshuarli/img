# Prerelease delivery

The manual release workflow follows the ish conventions: dispatch only from the
repository's default branch, build its immutable triggering SHA, run the complete
functional suite on three native runners, and publish `release-<full-sha>` only
after all jobs pass. Existing tags/assets are refused. Top-level permissions are
empty; only the publication job can write repository contents.

CI runs minimal-feature checks, Clippy and tests on macOS ARM64 and Linux
ARM64/x86-64 using the pinned nightly. Linux tests run unprivileged in the
repository's Alpine image, so permission tests exercise real denial. Native
codec/color tools are development references; the released binaries use the
pure-Rust production graph. Clippy's current warnings remain visible.

```sh
make ci
make release TARGET=aarch64-apple-darwin
# In the Linux image, choose its native musl target:
make release TARGET=aarch64-unknown-linux-musl
gh workflow run release.yml --ref master
```

`make release` builds the normal production feature set and verifies actual
output fidelity and linkage. Linux artifacts have neither PT_INTERP nor
DT_NEEDED; static PIE relocation sections are permitted. macOS links only OS
libraries. Publication verifies the exact expected artifact set, then packages
three ZIPs and a corresponding-source archive with a sorted SHA256SUMS. The ZIPs
include license notices. Source includes the exact tree, Cargo.lock and vendored
locked dependencies with their upstream notices; it builds without a registry:

```sh
tar -xzf rimage-source.tar.gz
cd rimage
cargo build --offline --locked --profile dist --features build-binary
```

Use Python 3.11+ for release helpers. Toolchain/linker installation is separate
from vendored crate availability. Linux release builds require the repository's
musl/mold environment. See [license provenance](PROVENANCE.md) before distributing
or modifying the combined AGPL work.

The private repository requires authenticated GitHub access. `~/usr` installs
the latest published SHA-named prerelease using `gh`; it checks the selected ZIP
against its manifest before installing and retains previous bytes on failure.
The sync tool dependency installs gh, but does not log in or change repository
visibility. Validate an installation through the public sync entrypoint in a
scratch HOME before using it on a host.

After downloading a release, verify all packages before extraction:

```sh
sha256sum -c SHA256SUMS
# macOS:
shasum -a 256 -c SHA256SUMS
```

Checksums bind downloaded bytes to the release manifest; they are not a separate
signature authority. Roll back by installing a previously verified release.
Never move an existing SHA-named tag or overwrite assets; fixes get a new commit
and release. Campaign performance/environment limitations remain recorded in
[plan.md](../plan.md); a working prerelease does not establish those measurements.
