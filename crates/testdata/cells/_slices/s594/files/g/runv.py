#!/usr/bin/env python3
# runv.py <bin> <tag> <dir>...: vita only on every cell -> <cell>.<tag> (stdout+stderr, then rc=N)
import sys, os, glob, subprocess
from concurrent.futures import ThreadPoolExecutor
V, T = sys.argv[1], sys.argv[2]
files = []
for d in sys.argv[3:]: files += sorted(glob.glob(os.path.join(d, '*.sv')))
def one(f):
    d, b = os.path.dirname(f), os.path.basename(f)
    try:
        p = subprocess.run([V, b], cwd=d, capture_output=True, text=True, timeout=120)
        o, rc = p.stdout + p.stderr, p.returncode
    except subprocess.TimeoutExpired:
        o, rc = 'TIMEOUT\n', -9
    open(f[:-3] + '.' + T, 'w').write(o + f'rc={rc}\n')
with ThreadPoolExecutor(6) as ex: list(ex.map(one, files))
print(T, len(files))
