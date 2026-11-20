import sys
sys.dont_write_bytecode = True
import contextlib, importlib.machinery, importlib.util, io, os, pathlib, subprocess, sys, tempfile, unittest

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


if __name__ == '__main__':
    unittest.main()
