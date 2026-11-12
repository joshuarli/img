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
and release. Performance and environment limitations remain recorded in
[benchmark evidence](../BENCHMARKS.md#remaining-proofs); a working prerelease does not establish those measurements.

If hosted runners cannot start, a maintainer can run the same checks locally,
package the three verified binaries and `scripts/package-source` output, then
publish a new SHA tag through authenticated gh. Release notes must disclose the
local build environments and the unavailable hosted result. This does not turn
a blocked Actions run into a pass or authorize billing changes.

## First published prerelease

[release-3e9aacfa5091a6e77ac1365b02511fb1752c5b71](https://github.com/joshuarli/rimage/releases/tag/release-3e9aacfa5091a6e77ac1365b02511fb1752c5b71)
was published from local verified builds. GitHub rejected the
[hosted release run](https://github.com/joshuarli/rimage/actions/runs/43174414030)
before executing a step: account payments failed or the spending limit needs
increasing. No billing setting or repository visibility was changed. Once the
account can start runners, rerun CI and dispatch the release workflow from the
default branch to verify the hosted path at a new commit.

Local acceptance includes 163 macOS tests and 161 tests per Linux architecture
(x86-64 under emulation), plus the final CI entrypoint on macOS and Linux ARM64.
All three release entrypoints pass linkage and actual-byte identity checks.
The 84,590,923-byte source archive builds offline and passes the macOS smoke
check. All uploaded archive digests match SHA256SUMS; the GitHub release is a
published prerelease with exactly three binary ZIPs, source and the manifest.

`~/usr` commit `4695d2c` adds the authenticated downloader, gh dependency and
supported default targets. Its 34 installer, 17 graph, nine DAG, eight integration
and 22 FFmpeg tests pass, as do isolated bootstrap and doctor checks. Bootstrap
isolation unsets inherited XDG paths so it exercises the scratch installation.
Ruff passes; ty still reports 17 diagnostics on existing unrelated code/imports,
with none in the new downloader. A real fresh scratch-home install, repeat
install and installed-binary identity check pass against the published release;
authentication for that isolated home was supplied in memory from the existing
gh login. The owner's live home installation was not changed.
