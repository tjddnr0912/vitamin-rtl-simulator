#!/usr/bin/env python3
# lens_run.py DIR TOOL BIN [SEP] -- one-shot BIN on DIR/<id>.sv -> DIR/<id>.TOOL.txt; with SEP also staged
# (SEP/vcmp -> velab -> vrun) -> DIR/<id>.TOOL_st.txt. Trailer `rc=N` like run.py.
import os, re, subprocess, sys
from concurrent.futures import ThreadPoolExecutor
D, tool, b = sys.argv[1], sys.argv[2], sys.argv[3]; sep = sys.argv[4] if len(sys.argv) > 4 else None
ENV = dict(os.environ, DEVELOPER_DIR='/Library/Developer/CommandLineTools')
def sh(a):
    r = subprocess.run(a, cwd=D, capture_output=True, text=True, env=ENV, timeout=300); return r.stdout + r.stderr, r.returncode
def one(c):
    o, rc = sh([b, f'{c}.sv']); open(f'{D}/{c}.{tool}.txt', 'w').write(o + f'\nrc={rc}\n')
    if sep:
        vu, ve = f'{D}/{c}.{tool}.vu', f'{D}/{c}.{tool}.velab'
        t, r = sh([f'{sep}/vcmp', f'{c}.sv', '-o', vu])
        if r == 0:
            t2, r = sh([f'{sep}/velab', vu, '-o', ve]); t += t2
            if r == 0: t3, r = sh([f'{sep}/vrun', ve]); t += t3
        for p in (vu, ve):
            if os.path.exists(p): os.remove(p)
        open(f'{D}/{c}.{tool}_st.txt', 'w').write(t + f'\nrc={r}\n')
ids = sorted(f[:-3] for f in os.listdir(D) if f.endswith('.sv'))
with ThreadPoolExecutor(4) as ex: list(ex.map(one, ids))
print(f'{tool}: {len(ids)} cells in {D}')
