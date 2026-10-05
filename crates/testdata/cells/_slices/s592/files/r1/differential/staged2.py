#!/usr/bin/env python3
# staged2.py TOOL SEPDIR [REGEX] -- vcmp -> velab -> vrun per cell with SEPDIR/{vcmp,velab,vrun};
# compares (sorted @ lines, sorted error lines, exit code) with the one-shot c/<id>.<TOOL>.txt.
import os, re, subprocess, sys
from concurrent.futures import ThreadPoolExecutor
G = '/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s592/r1/differential'; D = G + '/c'
tool, sep = sys.argv[1], sys.argv[2]; rx = sys.argv[3] if len(sys.argv) > 3 else '.*'
OUT = f'{G}/staged2/{tool}'; os.makedirs(OUT, exist_ok=True)
ENV = dict(os.environ, DEVELOPER_DIR='/Library/Developer/CommandLineTools')
ERR = re.compile(r'^(?:(\S+):(\d+):(\d+): )?(error|fatal)\[(VITA-\S+)\] (\S+): (.*)$')
def sh(args):
    r = subprocess.run(args, cwd=D, capture_output=True, text=True, env=ENV, timeout=300)
    return r.stdout + r.stderr, r.returncode
def staged(cid):
    vu, ve = f'{OUT}/{cid}.vu', f'{OUT}/{cid}.velab'
    for p in (vu, ve):
        if os.path.exists(p): os.remove(p)
    o1, r1 = sh([f'{sep}/vcmp', f'{cid}.sv', '-o', vu]); txt = o1; rc = r1
    if r1 == 0:
        o2, r2 = sh([f'{sep}/velab', vu, '-o', ve]); txt += o2; rc = r2
        if r2 == 0:
            o3, r3 = sh([f'{sep}/vrun', ve]); txt += o3; rc = r3
    open(f'{OUT}/{cid}.txt', 'w').write(txt + f'\nrc={rc}\n')
def key(t):
    m = re.search(r'\nrc=(-?\d+)\n$', t); rc = int(m.group(1)) if m else 99
    at = sorted(l.strip() for l in t.split('\n') if l.startswith('@'))
    errs = sorted(l for l in t.split('\n') if ERR.match(l))
    return (tuple(at), tuple(errs), rc)
ids = sorted(f[:-3] for f in os.listdir(D) if f.endswith('.sv') and not f.endswith('.sv2v.v') and re.fullmatch(rx, f[:-3]))
with ThreadPoolExecutor(4) as ex: list(ex.map(staged, ids))
same = 0; diff = []
for c in ids:
    a = key(open(f'{D}/{c}.{tool}.txt').read()); b = key(open(f'{OUT}/{c}.txt').read())
    if a == b: same += 1
    else: diff.append((c, a, b))
print(f'{tool}: staged == one-shot on {same}/{len(ids)} cells')
for c, a, b in diff:
    print('DIFF', c); print('   one-shot:', a); print('   staged  :', b)
