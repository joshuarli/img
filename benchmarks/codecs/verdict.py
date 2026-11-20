"""Budget verdicts shared by the comparison reports; budgets live in benchmarks/budgets.json.

A case above the per-case limit needs a disposition: BENCH_DISPOSITIONS names a JSON file mapping
metric -> case label -> reason. Undisposed cases fail the verdict like a geomean regression."""
import json, math, os, pathlib, random, statistics

BUDGETS = json.loads((pathlib.Path(__file__).resolve().parents[1] / 'budgets.json').read_text())


def dispositions():
    path = os.environ.get('BENCH_DISPOSITIONS')
    return json.loads(pathlib.Path(path).read_text()) if path else {}


def judge(metric, cases, disposed=None):
    """cases: list of (label, new/base ratio). Returns a verdict dict."""
    budget = BUDGETS['geomean'][metric]
    reasons = (dispositions() if disposed is None else disposed).get(metric, {})
    logs = [math.log(r) for _, r in cases]
    rng = random.Random(0)
    boot = sorted(math.exp(statistics.fmean(rng.choices(logs, k=len(logs)))) for _ in range(2000))
    geomean, lo, hi = math.exp(statistics.fmean(logs)), boot[50], boot[1949]
    status = 'regression' if lo > budget else 'within' if hi <= budget else 'inconclusive'
    over = sorted(((label, round(r, 4)) for label, r in cases if r > BUDGETS['per_case']), key=lambda x: -x[1])
    return dict(metric=metric, n=len(cases), geomean=round(geomean, 4), ci=[round(lo, 4), round(hi, 4)], budget=budget, status=status,
                cases_over=over, dispositions={label: reasons[label] for label, _ in over if label in reasons},
                undisposed=[label for label, _ in over if label not in reasons])


def write(path, verdicts):
    """Writes the verdicts; returns True when any metric regressed or has undisposed cases over the per-case limit."""
    pathlib.Path(path).write_text(json.dumps(verdicts, indent=1))
    for v in verdicts:
        over = f'; {len(v["cases_over"])} cases over {BUDGETS["per_case"]} ({len(v["undisposed"])} undisposed): ' + ', '.join(f'{l} {r}' for l, r in v['cases_over'][:5]) if v['cases_over'] else ''
        print(f'VERDICT {v["metric"]:15} {v["status"]:12} geomean {v["geomean"]:.3f} [{v["ci"][0]:.3f}, {v["ci"][1]:.3f}] budget {v["budget"]}{over}')
        for label in v['undisposed']:
            print(f'  UNDISPOSED {v["metric"]} {label}: add a reason under BENCH_DISPOSITIONS')
    return any(v['status'] == 'regression' or v['undisposed'] for v in verdicts)
