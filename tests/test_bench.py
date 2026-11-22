import sys
sys.dont_write_bytecode = True
import importlib.machinery, importlib.util, os, pathlib, subprocess, sys, tempfile, unittest

ROOT = pathlib.Path(__file__).resolve().parent.parent
loader = importlib.machinery.SourceFileLoader('bench', str(ROOT / 'scripts/bench'))
spec = importlib.util.spec_from_loader('bench', loader)
bench = importlib.util.module_from_spec(spec)
loader.exec_module(bench)


def names(chosen): return [w['name'] for w, _ in chosen]


class Selection(unittest.TestCase):
    def test_tier_is_a_ceiling_and_picks_the_highest_level_below_it(self):
        chosen, excluded = bench.select('change')
        self.assertIn('workflow-scored', names(chosen))
        self.assertNotIn('jpeg-decode', names(chosen))
        self.assertIn('jpeg-decode', [n for n, _ in excluded])
        self.assertEqual(dict((w['name'], l) for w, l in bench.select('scenario')[0])['workflow-scored'], 'scenario')
        self.assertEqual(names(bench.select('edit')[0]), ['harness'])

    def test_names_and_areas_are_distinct_selectors(self):
        self.assertEqual(names(bench.select('change', only=['batch'])[0]), ['batch'])
        self.assertEqual(sorted(names(bench.select('full', areas=['decode'])[0])), ['avif-decode', 'jpeg-decode', 'png-decode'])
        with self.assertRaises(SystemExit): bench.select('change', only=['decode'])
        with self.assertRaises(SystemExit): bench.select('change', areas=['batch-throughput'])

    def test_unknown_and_empty_selections_fail(self):
        with self.assertRaises(SystemExit): bench.select('change', only=['nope'])
        with self.assertRaises(SystemExit): bench.select('change', skip=['nope'])
        with self.assertRaises(SystemExit): bench.select('change', only=['png-decode'])
        with self.assertRaises(SystemExit): bench.select('edit', skip=['harness'])

    def test_skips_are_reported_as_exclusions(self):
        chosen, excluded = bench.select('change', skip=['batch'])
        self.assertNotIn('batch', names(chosen))
        self.assertIn(('batch', 'skipped by --skip'), excluded)

    def test_corpus_sets_are_deterministic_dev_files(self):
        smoke = bench.corpus_set('smoke')
        self.assertEqual(len(smoke), len(set(smoke)))
        self.assertTrue(set(smoke) <= set(bench.corpus_set('scenario')) <= set(bench.corpus_set('dev')))
        self.assertFalse(bench.UNSUPPORTED & set(bench.corpus_set('dev')))

    def test_holdout_is_selected_only_explicitly_and_never_overlaps_dev(self):
        dev = set(bench.corpus_set('dev'))
        holdout = set(bench.corpus_set('scenario', split='holdout'))
        self.assertTrue(holdout)
        self.assertFalse(dev & holdout)
        self.assertTrue(set(bench.corpus_set('scenario')) <= dev)


class Guards(unittest.TestCase):
    def run_bench(self, *args, env=None):
        return subprocess.run([sys.executable, str(ROOT / 'scripts/bench'), *args], capture_output=True, text=True,
                              env={**os.environ, **(env or {})}, cwd=ROOT)

    def test_full_tier_needs_explicit_confirmation(self):
        p = self.run_bench('run', '--tier', 'full', '--only', 'historical', '--base', 'HEAD')
        self.assertNotEqual(p.returncode, 0)
        self.assertIn('manual-only', p.stderr)

    def test_failed_preflight_runs_nothing(self):
        with tempfile.TemporaryDirectory() as empty:
            runs = bench.WORK / 'runs'
            before = set(runs.iterdir()) if runs.exists() else set()
            p = self.run_bench('run', '--only', 'workflow-lossless', '--base', 'HEAD', env={'BENCH_CORPUS': empty})
            after = set(runs.iterdir()) if runs.exists() else set()
        self.assertEqual(p.returncode, 2, p.stdout + p.stderr)
        self.assertIn('qualification corpus', p.stdout)
        self.assertIn('nothing ran', p.stdout)
        self.assertEqual(before, after)

    def test_dry_run_prints_commands_and_writes_nothing(self):
        runs = bench.WORK / 'runs'
        before = set(runs.iterdir()) if runs.exists() else set()
        p = self.run_bench('run', '--tier', 'scenario', '--only', 'workflow-scored', '--base', 'HEAD', '--dry-run')
        after = set(runs.iterdir()) if runs.exists() else set()
        self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
        self.assertIn('benchmarks/codecs/workflow.py', p.stdout)
        self.assertIn('WORKFLOW_SCORES=90', p.stdout)
        self.assertIn('WORKFLOW_VERIFIER=', p.stdout)
        self.assertEqual(before, after)

    def test_decode_report_names_candidate_and_reference(self):
        p = self.run_bench('run', '--tier', 'scenario', '--only', 'jpeg-decode', '--base', 'HEAD', '--dry-run')
        self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
        self.assertRegex(p.stdout, r'jpeg/decode-report.py \S+run.json new base')

    def test_holdout_dry_run_uses_only_holdout_inputs(self):
        p = self.run_bench('run', '--tier', 'scenario', '--only', 'workflow-lossless', '--base', 'HEAD', '--split', 'holdout', '--dry-run')
        self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
        dev = set(bench.corpus_set('dev'))
        named = {pathlib.Path(w).name for w in p.stdout.split() if w.endswith(('.png', '.jpg'))}
        self.assertTrue(named)
        self.assertFalse(named & dev)



sys.path.insert(0, str(ROOT / 'benchmarks/codecs'))
import verdict


class Budgets(unittest.TestCase):
    def test_ci_relative_to_budget_decides_status(self):
        self.assertEqual(verdict.judge('wall', [(str(i), 0.9 + i * 0.001) for i in range(10)])['status'], 'within')
        self.assertEqual(verdict.judge('wall', [(str(i), 1.2 + i * 0.001) for i in range(10)])['status'], 'regression')
        self.assertEqual(verdict.judge('wall', [('a', 0.8), ('b', 1.3), ('c', 0.95), ('d', 1.25)])['status'], 'inconclusive')

    def test_single_cases_over_the_per_case_limit_are_listed(self):
        v = verdict.judge('bytes', [('fine', 1.0), ('bad', 1.2)], disposed={})
        self.assertEqual(v['cases_over'], [('bad', 1.2)])

    def test_cases_over_the_limit_need_a_disposition(self):
        cases = [('fine', 1.0), ('bad', 1.2)]
        with tempfile.TemporaryDirectory() as d:
            path = pathlib.Path(d) / 'v.json'
            self.assertTrue(verdict.write(path, [verdict.judge('bytes', cases, disposed={})]))
            disposed = verdict.judge('bytes', cases, disposed={'bytes': {'bad': 'named size-for-speed tradeoff'}})
            self.assertEqual(disposed['undisposed'], [])
            self.assertEqual(disposed['dispositions'], {'bad': 'named size-for-speed tradeoff'})



FAKE_IMG = r"""#!/usr/bin/env python3
import json, os, subprocess, sys
tool, state = os.path.basename(sys.argv[0]), os.environ['FAKE_STATE']
counter = os.path.join(state, tool + '.calls')
call = int(open(counter).read()) if os.path.exists(counter) else 0
open(counter, 'w').write(str(call + 1))
plan = json.loads(os.environ['FAKE_PLAN'])[tool]
entry = plan[min(call, len(plan) - 1)]
content = lambda source: (entry if isinstance(entry, str) else entry.get(os.path.basename(source), entry['*'])) + ':' + os.path.basename(source)
spawn = os.environ.get('FAKE_SPAWN')
if spawn and not os.path.exists(os.path.join(state, 'spawned')):
    open(os.path.join(state, 'spawned'), 'w').close()
    subprocess.Popen([spawn, '1.5'], start_new_session=True)
args = sys.argv[2:]
if '--out-dir' in args:
    dest = args[args.index('--out-dir') + 1]; os.makedirs(dest)
    sources = [a for a in args[:args.index('--out-dir')]]
    results = []
    for source in sources:
        out = os.path.join(dest, os.path.basename(source))
        open(out, 'w').write(content(source))
        results.append(dict(input=source, output=out, outputBytes=os.path.getsize(out), format='png'))
    print(json.dumps(dict(schemaVersion=1, results=results, failures=[])))
else:
    open(args[args.index('-o') + 1], 'w').write(content(args[0]))
"""

FAKE_VERIFIER = r"""#!/usr/bin/env python3
import json, os, sys
log = open(os.path.join(os.environ['FAKE_STATE'], 'verified.log'), 'a')
for line in sys.stdin:
    source, output, requirement = line.rstrip('\n').split('\t')
    log.write(line)
    good = open(output).read().startswith('good')
    print(json.dumps(dict(source=source, output=output, requirement=requirement, exact=None, score=95.0 if good else 10.0, accepted=good, error=None)))
"""


class Evidence(unittest.TestCase):
    """End-to-end runs of the workflow and batch runners against a scripted img and verifier, ungated."""

    def setUp(self):
        self.dir = pathlib.Path(tempfile.mkdtemp(dir=os.environ.get('TMPDIR')))
        self.addCleanup(__import__('shutil').rmtree, self.dir)
        self.state = self.dir / 'state'; self.state.mkdir()
        for name, text in (('new', FAKE_IMG), ('base', FAKE_IMG), ('verify', FAKE_VERIFIER)):
            (self.dir / name).write_text(text); (self.dir / name).chmod(0o755)
        self.inputs = self.dir / 'in'; self.inputs.mkdir()
        for name in ('a.png', 'b.png'):
            (self.inputs / name).write_text('source')

    def env(self, plan, **extra):
        return {**os.environ, 'BENCH_GATE': '0', 'BENCH_DISQUALIFYING': 'no-such-process', 'FAKE_STATE': str(self.state),
                'FAKE_PLAN': __import__('json').dumps(plan), 'WORKFLOW_FORMATS': 'png', 'WORKFLOW_SCORES': '90',
                'WORKFLOW_VERIFIER': str(self.dir / 'verify'), 'BATCH_VERIFIER': str(self.dir / 'verify'), **extra}

    def script(self, name, *args, env):
        return subprocess.run([sys.executable, str(ROOT / 'benchmarks/codecs' / name), *map(str, args)], capture_output=True, text=True, env=env)

    def workflow(self, plan, rounds, **extra):
        env = self.env(plan, **extra)
        run = self.script('workflow.py', self.dir / 'run.json', rounds, 'balanced', f'new={self.dir / "new"}', f'base={self.dir / "base"}', '--',
                          self.inputs / 'a.png', env=env)
        self.assertEqual(run.returncode, 0, run.stdout + run.stderr)
        report = self.script('workflow-report.py', self.dir / 'run.json', 'new', 'base', env=env)
        return __import__('json').loads((self.dir / 'run.json').read_text()), report

    def verified_lines(self):
        return (self.state / 'verified.log').read_text().splitlines()

    def test_an_unverified_earlier_round_fails_the_case_even_when_the_last_round_verifies(self):
        doc, report = self.workflow({'new': ['bad', 'good'], 'base': ['good']}, rounds=2)
        self.assertEqual([x['verified'] for x in doc['samples'] if x['tool'] == 'new'], [False, True])
        self.assertIn('failed new a.png png 90 verifier rejected a timed output', report.stdout)
        self.assertEqual(report.returncode, 4, report.stdout)

    def test_zero_jointly_successful_cases_is_insufficient_evidence(self):
        _, report = self.workflow({'new': ['bad'], 'base': ['good']}, rounds=1)
        self.assertIn('INSUFFICIENT EVIDENCE', report.stdout)
        self.assertIn('new failed cases base completed', report.stdout)
        self.assertEqual(report.returncode, 4)

    def test_identical_outputs_are_verified_once_and_distinct_outputs_each(self):
        doc, report = self.workflow({'new': ['good', 'good-variant', 'good'], 'base': ['good']}, rounds=3)
        self.assertEqual(report.returncode, 0, report.stdout + report.stderr)
        self.assertEqual(len(self.verified_lines()), 2)
        self.assertEqual(len({x['sha256'] for x in doc['samples']}), 2)
        self.assertTrue(all(x['verified'] and pathlib.Path(x['output']).exists() for x in doc['samples']))

    def test_a_round_overlapping_a_build_is_discarded_with_its_disposition(self):
        build = self.dir / 'fakebuild'; build.symlink_to('/bin/sleep')
        doc, report = self.workflow({'new': ['good'], 'base': ['good']}, rounds=1, BENCH_DISQUALIFYING='fakebuild', FAKE_SPAWN=str(build))
        self.assertRegex(doc['dispositions'][0]['disposition'], r'^discarded: overlapped fakebuild \(pid \d+\)$')
        self.assertEqual(doc['dispositions'][-1], {'attempt': len(doc['dispositions']), 'disposition': 'kept', 'round': 0})
        self.assertEqual(doc['discarded_rounds'], len(doc['dispositions']) - 1)
        self.assertEqual({x['round'] for x in doc['samples']}, {0})
        self.assertIn('overlapped fakebuild', report.stdout)

    def batch(self, plan, rounds):
        return self.script('batch.py', self.dir / 'batch.json', rounds, self.inputs, 'lossless', f'new={self.dir / "new"}', f'base={self.dir / "base"}',
                           '--', 1, 2, env=self.env(plan))

    def test_batch_fails_when_any_timed_run_produced_an_unverified_output(self):
        p = self.batch({'new': ['good', {'a.png': 'bad', '*': 'good'}, 'good'], 'base': ['good']}, rounds=2)
        self.assertEqual(p.returncode, 1, p.stdout + p.stderr)
        self.assertIn('equivalent-work check failed', p.stdout)
        self.assertIn('verified 2/3 distinct outputs covering 15/16 timed outputs', p.stdout)
        self.assertRegex(p.stdout, r'new -j 2 round 0 lacks \S+/a.png\n')

    def test_batch_with_no_commonly_verified_input_is_insufficient_evidence(self):
        p = self.batch({'new': ['bad'], 'base': ['good']}, rounds=1)
        self.assertEqual(p.returncode, 4, p.stdout + p.stderr)
        self.assertIn('INSUFFICIENT EVIDENCE', p.stdout)

    def test_batch_verifies_each_distinct_output_once(self):
        p = self.batch({'new': ['good'], 'base': ['good']}, rounds=3)
        self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
        self.assertEqual(len(self.verified_lines()), 2)


if __name__ == '__main__':
    unittest.main()
