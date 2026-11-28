#!/bin/sh
# Sequential qualification session; every workload is gated and build-watched by evidence.py.
set -u
R=/Users/josh/d/img; B=$R/.work/bench/bin; Q=$R/.work/qual; mkdir -p $Q
export TMPDIR=$R/.work/tmp IMG_MEMORY_BUDGET=2660000000
CAND=$B/img-candidate-b89205c4e400; BASE=$B/img-f0ceea6aae093ba50d747dca51737458257a2caf; ZEN=$R/.work/refs/zen-5424df8
V=$R/.work/bench/loops/target-img/release/verify
export WORKFLOW_VERIFIER=$V BATCH_VERIFIER=$V
cd $R
step() { name=$1; shift; mkdir -p $Q/$name; echo "== $name $(date -u +%H:%M:%S)"; "$@" > $Q/$name/log.txt 2>&1; echo "$name exit $? $(date -u +%H:%M:%S)"; }
SCEN=$(python3 -c "
import importlib.machinery,importlib.util
l=importlib.machinery.SourceFileLoader('b','scripts/bench'); m=importlib.util.module_from_spec(importlib.util.spec_from_loader('b',l)); l.exec_module(m)
print(' '.join(str(m.CORPUS/f) for f in m.corpus_set('scenario')))")
# Q1 single-image workflow latency vs B, 5 rounds
step q1-workflow-base env WORKFLOW_FORMATS=jpeg,png,avif WORKFLOW_SCORES=90 python3 benchmarks/codecs/workflow.py $Q/q1-workflow-base/run.json 5 balanced new=$CAND base=$BASE -- $SCEN
python3 benchmarks/codecs/workflow-report.py $Q/q1-workflow-base/run.json new base > $Q/q1-workflow-base/report.txt 2>&1
# Q2 single-image workflow vs Zen, 3 rounds
step q2-workflow-zen env WORKFLOW_FORMATS=jpeg,png,avif WORKFLOW_SCORES=90 python3 benchmarks/codecs/workflow.py $Q/q2-workflow-zen/run.json 3 balanced new=$CAND zen=$ZEN -- $SCEN
python3 benchmarks/codecs/workflow-report.py $Q/q2-workflow-zen/run.json new zen > $Q/q2-workflow-zen/report.txt 2>&1
# Q3 compact scheduler, with and without scoring parallelism, 16 large JPEGs, 3 rounds
step q3-scheduler python3 benchmarks/codecs/batch.py $Q/q3-scheduler/run.json 3 .work/sets/large jpeg90 new=$CAND new-nocompact=$B/img-candidate-nocompact pre=$B/img-77eb60a pre-nocompact=$B/img-77eb60a-nocompact -- 1 2 4
# Q4 mixed batch vs B, 3 rounds
step q4-mix-base python3 benchmarks/codecs/batch.py $Q/q4-mix-base/run.json 3 .work/sets/mix keep90 new=$CAND base=$BASE -- 1 2 4
# Q5 allocator A/B on a sustained mixed batch (mix three times, 81 inputs)
rm -rf .work/sets/mix3; mkdir -p .work/sets/mix3; for p in a b c; do for f in .work/sets/mix/*; do cp $f .work/sets/mix3/$p-$(basename $f); done; done
step q5-allocator python3 benchmarks/codecs/batch.py $Q/q5-allocator/run.json 3 .work/sets/mix3 keep90 new=$CAND sysalloc=$B/img-sysalloc -- 1 4
# Q6 mixed batch vs Zen, 3 rounds
step q6-mix-zen python3 benchmarks/codecs/batch.py $Q/q6-mix-zen/run.json 3 .work/sets/mix keep90 new=$CAND zen=$ZEN -- 1 4
echo done $(date -u +%H:%M:%S)
