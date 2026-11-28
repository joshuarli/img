# img scratch archive (2026-11-27)

Orphan branch holding small text that existed only under `.work/` when this
repository went on vacation. It is not a build input: `docs/roadmap.md` Status
on the default branch gives the rebuild order.

- `tmp/qualify.sh`, `tmp/refbuilds.sh`, `tmp/variants.sh`: hand-written recipes
  for the reference/variant binaries and the q1-q6 qualification session.
- `qual/q*/`: run records, reports, verdicts and logs from that session
  (candidate b89205c against B f0ceea6 and Zen 5424df8, 2026-11-22/23).
- `sets.sha`: sha256 manifest for the generated batch-sets fresh inputs.
- `tmp/*.log`, `tmp/avifdev.*`, `tmp/avif-dev.txt`: session logs and dev lists.

Committed conclusions live in `docs/CODECS.md`,
`benchmarks/results/checkpoints/2026-11-20/` and
`benchmarks/results/codecs/avif/search-probe-2026-11-22.json`. No credentials
were archived. Large binaries, corpora and build outputs are not here; rebuild
them with the commands in `docs/roadmap.md` Status.
