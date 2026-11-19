import pathlib
import subprocess
import sys

stock, patched, generated, decfix, scratch = sys.argv[1:]
paths = sorted(pathlib.Path('benchmarks/corpus/validation').glob('*.jpg'))
paths += sorted(pathlib.Path(decfix).glob('*.jpg'))
paths += sorted(pathlib.Path(generated).glob('*.jpg'))
scratch = pathlib.Path(scratch)
scratch.mkdir(parents=True, exist_ok=True)
count = 0
for index, path in enumerate(paths):
    data = path.read_bytes()
    for fraction in (100, 50, 90):
        sample = path
        if fraction != 100:
            sample = scratch / f'{index}-{fraction}.jpg'
            sample.write_bytes(data[:len(data) * fraction // 100])
        results = [subprocess.run([binary, 'zune-dump', str(sample)], capture_output=True)
                   for binary in (stock, patched)]
        count += 1
        if (results[0].returncode, results[0].stdout, results[0].stderr) != (results[1].returncode, results[1].stdout, results[1].stderr):
            print(f'mismatch: {path} at {fraction}%')
            sys.exit(1)
print(f'originals={len(paths)} compared={count} mismatches=0')
