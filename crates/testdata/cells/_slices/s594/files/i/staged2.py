#!/usr/bin/env python3
# staged.py SEPDIR ONESHOT_TAG OUT_TAG: vcmp -> velab -> vrun per cell of every s594 set (+ i/probe i/dump i/pin);
# writes <cell>.<OUT_TAG>; compares sm.py-normalized text with <cell>.<ONESHOT_TAG>
import os, re, subprocess, sys, glob
from concurrent.futures import ThreadPoolExecutor
S='/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s594'
sep, one, out = sys.argv[1], sys.argv[2], sys.argv[3]
src = open(f'{S}/g/sm.py').read().split('for f in sorted(os.listdir(d)):')[0].replace('d = sys.argv[1]', 'd = None').replace('tags = sys.argv[2].split(",") if len(sys.argv) > 2 else ["pre", "ivl", "s2v", "vl"]', '')
ns = {}; exec(src, ns); norm = ns['norm']
dirs=[f'{S}/g/c1',f'{S}/g/c2',f'{S}/g/c3',f'{S}/g/c4']+sorted(d for d in glob.glob(f'{S}/g/old/*') if os.path.isdir(d))+[f'{S}/a/cells',f'{S}/a/cells2',f'{S}/a/cells3',f'{S}/a/cells4',f'{S}/a/cells5',f'{S}/i/probe',f'{S}/i/dump',f'{S}/i/pin',f'{S}/a/ivd',f'{S}/i/r2']+sorted(glob.glob(f'{S}/i/r2/lens/*'))
W = f'{S}/i/stg/{out}'; os.makedirs(W, exist_ok=True)
ENV = dict(os.environ, DEVELOPER_DIR='/Library/Developer/CommandLineTools')
def sh(args, cwd):
    try:
        r = subprocess.run(args, cwd=cwd, capture_output=True, text=True, env=ENV, timeout=120)
        return r.stdout + r.stderr, r.returncode
    except subprocess.TimeoutExpired:
        return 'TIMEOUT\n', -9
def staged(f):
    d, b = os.path.dirname(f), os.path.basename(f)[:-3]
    tag = (os.path.relpath(d, S) + '_' + b).replace('/', '_')
    vu, ve = f'{W}/{tag}.vu', f'{W}/{tag}.velab'
    for p in (vu, ve):
        if os.path.exists(p): os.remove(p)
    t, rc = sh([f'{sep}/vcmp', b + '.sv', '-o', vu], d)
    if rc == 0:
        o, rc = sh([f'{sep}/velab', vu, '-o', ve], d); t += o
        if rc == 0:
            o, rc = sh([f'{sep}/vrun', ve], d); t += o
    open(f'{d}/{b}.{out}', 'w').write(t + f'rc={rc}\n')
    for p in (vu, ve):
        if os.path.exists(p): os.remove(p)
files = [f for d in dirs for f in sorted(glob.glob(f'{d}/*.sv'))]
with ThreadPoolExecutor(6) as ex: list(ex.map(staged, files))
same = 0; diffs = []
for f in files:
    b = f[:-3]
    a, s = norm(f'{b}.{one}', 'pre'), norm(f'{b}.{out}', 'pre')
    if a == s: same += 1
    else: diffs.append((os.path.relpath(b, S), a, s))
print(f'{out}: staged == one-shot({one}) on {same}/{len(files)}')
for c, a, s in diffs: print('DIFF', c, '\n   one-shot:', a[:150], '\n   staged  :', s[:150])
