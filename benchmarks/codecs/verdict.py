"""Budget verdicts shared by the comparison reports; budgets live in benchmarks/budgets.json."""
import json, math, pathlib, random, statistics

BUDGETS = json.loads((pathlib.Path(__file__).resolve().parents[1] / 'budgets.json').read_text())


def judge(metric, cases):
    """cases: list of (label, new/base ratio). Returns a verdict dict."""
    budget = BUDGETS['geomean'][metric]
    logs = [math.log(r) for _, r in cases]
    rng = random.Random(0)
    boot = sorted(math.exp(statistics.fmean(rng.choices(logs, k=len(logs)))) for _ in range(2000))
    geomean, lo, hi = math.exp(statistics.fmean(logs)), boot[50], boot[1949]
    status = 'regression' if lo > budget else 'within' if hi <= budget else 'inconclusive'
    over = sorted(((label, round(r, 4)) for label, r in cases if r > BUDGETS['per_case']), key=lambda x: -x[1])
    return dict(metric=metric, n=len(cases), geomean=round(geomean, 4), ci=[round(lo, 4), round(hi, 4)], budget=budget, status=status, cases_over=over)


def write(path, verdicts):
    pathlib.Path(path).write_text(json.dumps(verdicts, indent=1))
    for v in verdicts:
        over = f'; {len(v["cases_over"])} cases over {BUDGETS["per_case"]}: ' + ', '.join(f'{l} {r}' for l, r in v['cases_over'][:5]) if v['cases_over'] else ''
        print(f'VERDICT {v["metric"]:15} {v["status"]:12} geomean {v["geomean"]:.3f} [{v["ci"][0]:.3f}, {v["ci"][1]:.3f}] budget {v["budget"]}{over}')
    return any(v['status'] == 'regression' for v in verdicts)
