set -x
export TMPDIR=/Users/josh/d/img/.work/tmp CARGO_BUILD_JOBS=4 HOST_CC=clang HOST_CXX=clang++
B=/Users/josh/d/img/.work/bench/bin
while pgrep -f refbuilds.sh >/dev/null; do sleep 10; done
nocompact() { python3 -c "
import sys; p='src/runtime.rs'; s=open(p).read(); old='let obtainable=budget.free_now().saturating_add(reserved.bytes);'
assert s.count(old)==1; open(p,'w').write(s.replace(old,'let obtainable=budget.total;'))"; }
cd /Users/josh/d/img/.work/wt/sysalloc && git checkout -q src/main.rs && nocompact && git diff --stat && cargo build --locked --profile dist --features build-binary && cp target/dist/img $B/img-candidate-nocompact
echo nocompact-exit $?
cd /Users/josh/d/img && git worktree add -q --detach .work/wt/preparallel 77eb60a && cd .work/wt/preparallel && cargo build --locked --profile dist --features build-binary && cp target/dist/img $B/img-77eb60a
echo preparallel-exit $?
nocompact && git diff --stat && cargo build --locked --profile dist --features build-binary && cp target/dist/img $B/img-77eb60a-nocompact
echo preparallel-nocompact-exit $?
