set -x
export TMPDIR=/Users/josh/d/img/.work/tmp CARGO_BUILD_JOBS=4 HOST_CC=clang HOST_CXX=clang++
cd /Users/josh/d/img/.work/wt/zen-5424df8 && cargo build --locked --profile dist --features build-binary && cp target/dist/rimage /Users/josh/d/img/.work/refs/zen-5424df8 || cp target/dist/img /Users/josh/d/img/.work/refs/zen-5424df8
echo zen-exit $?
cd /Users/josh/d/img/.work/wt/sysalloc && cargo build --locked --profile dist --features build-binary && cp target/dist/img /Users/josh/d/img/.work/bench/bin/img-sysalloc
echo sysalloc-exit $?
