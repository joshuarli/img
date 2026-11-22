"""Round admission and output evidence shared by workflow.py and batch.py.

Rounds: a round is kept only when perf-gate-ready passes before it (unless gating is off), no disqualifying
process (a build, by default) runs at any point during it, and perf-gate-ready -1 passes after it. Every
attempt gets a recorded disposition. BENCH_GATE=0 disables the load gate but not the overlap watch;
BENCH_DISQUALIFYING overrides the comma list of process names that disqualify a round.

Outputs: every timed output is hashed after its process exits, outside the timed interval. The first output
with a given hash is moved into a content-addressed store and every distinct (source, hash, requirement) is
verified once, so each timed sample is either verified or byte-identical to a verified retained output."""
import hashlib, json, os, shutil, subprocess, threading, time

BUILDS = 'cargo,rustc,cc,c++,clang,clang++,gcc,g++,ld,ld64,mold,make,gmake,ninja,meson,cmake'


def sha256(path):
    h = hashlib.sha256()
    with open(path, 'rb') as f:
        for block in iter(lambda: f.read(1 << 20), b''):
            h.update(block)
    return h.hexdigest()


def gated():
    return os.environ.get('BENCH_GATE', '1') != '0'


def host_busy(*args):
    return subprocess.run(['perf-gate-ready', *args], capture_output=True).returncode != 0


class OverlapWatch:
    """Polls the process table while a round runs and remembers disqualifying processes that are not our ancestors."""

    def __init__(self, names=None, interval=1.0):
        listed = os.environ.get('BENCH_DISQUALIFYING', BUILDS) if names is None else names
        self.names, self.interval, self.seen = {n for n in listed.split(',') if n}, interval, {}
        self.stop = threading.Event()

    def poll(self):
        table = {}
        for line in subprocess.run(['ps', '-axo', 'pid=,ppid=,comm='], capture_output=True, text=True).stdout.splitlines():
            pid, ppid, comm = line.strip().split(None, 2)
            table[int(pid)] = (int(ppid), os.path.basename(comm))
        ancestors, pid = set(), os.getpid()
        while pid in table and pid not in ancestors:
            ancestors.add(pid); pid = table[pid][0]
        for pid, (_, comm) in table.items():
            if comm in self.names and pid not in ancestors:
                self.seen.setdefault(comm, pid)

    def __enter__(self):
        self.poll()
        def loop():
            while not self.stop.wait(self.interval):
                self.poll()
        self.thread = threading.Thread(target=loop, daemon=True); self.thread.start()
        return self

    def __exit__(self, *exc):
        self.stop.set(); self.thread.join(); self.poll()


def run_rounds(count, run_round, wait=20):
    """Calls run_round(index) until count rounds are kept. Returns (kept round lists, dispositions)."""
    kept, dispositions, attempt, gate = [], [], 0, gated()
    while len(kept) < count:
        attempt += 1
        if gate and host_busy():
            dispositions.append(dict(attempt=attempt, disposition='not started: host busy before the round'))
            time.sleep(wait); continue
        with OverlapWatch() as watch:
            samples = run_round(len(kept))
        if watch.seen:
            reason = 'discarded: overlapped ' + ', '.join(f'{n} (pid {p})' for n, p in sorted(watch.seen.items()))
        elif gate and host_busy('-1'):
            reason = 'discarded: host busy after the round'
        else:
            reason = None
        dispositions.append(dict(attempt=attempt, disposition=reason or 'kept', round=None if reason else len(kept)))
        if reason:
            print(f'attempt {attempt} {reason}', flush=True); continue
        kept.append(samples); print('round', len(kept), flush=True)
    return kept, dispositions


def discarded(dispositions):
    return sum(1 for d in dispositions if d['disposition'] != 'kept')


class Store:
    """Content-addressed retention of timed outputs, one file per distinct hash."""

    def __init__(self, root):
        self.root = root; os.makedirs(root, exist_ok=True)

    def retain(self, path, consume=True):
        digest = sha256(path)
        kept = os.path.join(self.root, digest + os.path.splitext(path)[1])
        if not os.path.exists(kept):
            (shutil.move if consume else shutil.copyfile)(path, kept)
        elif consume:
            os.unlink(path)
        return digest, kept


def verify(verifier, claims):
    """claims: iterable of (source, retained path, requirement); each distinct claim is checked once.
    Returns {claim: verifier record}."""
    distinct = sorted(set(claims))
    if not distinct:
        return {}
    lines = ''.join(f'{s}\t{p}\t{r}\n' for s, p, r in distinct)
    checked = subprocess.run([verifier], input=lines, capture_output=True, text=True, check=True).stdout.splitlines()
    return {claim: json.loads(line) for claim, line in zip(distinct, checked, strict=True)}
