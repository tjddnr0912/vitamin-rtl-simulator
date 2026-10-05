#!/usr/bin/env python3
# run.py TOOL[,TOOL..] [ids-prefix-filter]  -- runs each cell through the tools, caches raw output
# tools: iv (iverilog), sv (sv2v->iverilog), vl (verilator), pre, post (=$SCR/postT/vita), or a path tag
import os, re, subprocess, sys, json
from concurrent.futures import ThreadPoolExecutor
H = os.path.dirname(os.path.abspath(__file__))
D = H + '/cells'
SCR = os.path.dirname(H)
S580 = SCR + '/../s580'
SV2V = S580 + '/sv2v/sv2v-macOS/sv2v'
VITA = {'pre': SCR + '/pre/vita', 'post': SCR + '/postT/vita', 's2': SCR + '/post/vita', 'p2': SCR + '/post2/vita', 'p3': SCR + '/post3/vita', 'dbg': '/Users/seongwookjang/project/git/vitamin-rtl-simulator/target/debug/vita'}

def sh(cmd, timeout=300):
    try:
        r = subprocess.run(cmd, shell=True, capture_output=True, text=True, cwd=D, timeout=timeout)
        return r.stdout + r.stderr, r.returncode
    except subprocess.TimeoutExpired:
        return 'TIMEOUT', -9

def run(tool, cid):
    f = f'{cid}.sv'
    if tool == 'iv':
        out, rc = sh(f'iverilog -g2012 -o {cid}.vvp {f} && vvp -n {cid}.vvp')
    elif tool == 'sv':
        out, rc = sh(f'{SV2V} {f} > {cid}.sv2v.v && iverilog -g2012 -o {cid}.s.vvp {cid}.sv2v.v && vvp -n {cid}.s.vvp')
    elif tool == 'vl':
        out, rc = sh(f'rm -rf obj_{cid}; verilator --binary --timing -Wno-fatal -Wno-lint -Wno-style --top-module top {f} -Mdir obj_{cid} > {cid}.vlb.txt 2>&1; r=$?; cat {cid}.vlb.txt | grep -E "%Error" | head -3; [ $r = 0 ] && obj_{cid}/Vtop', 600)
    else:
        out, rc = sh(f'{VITA[tool]} {f}')
    open(f'{D}/{cid}.{tool}.txt', 'w').write(out + f'\nrc={rc}\n')

def verdict(tool, cid):
    p = f'{D}/{cid}.{tool}.txt'
    if not os.path.exists(p):
        return '?'
    t = open(p).read()
    m = re.search(r'\nrc=(-?\d+)\n$', t)
    rc = int(m.group(1)) if m else 99
    arms = sorted(set(a.strip().replace(' ', ':') for a in re.findall(rf'^{cid} (.+)$', t, re.M)))
    if arms and rc == 0:
        return '/'.join(arms)
    err = None
    for ln in t.split('\n'):
        mm = re.match(r'^(?:(\S+):(\d+):(\d+): )?(error|fatal)\[(VITA-\S+)\] (\S+): (.*)$', ln)
        if mm:
            err = f'ERR:{mm.group(5)}'
            break
        if re.search(r'%Error|error:|syntax error|sorry|Unable|unsupported|TIMEOUT|panic', ln, re.I):
            err = 'ERR:' + re.sub(r'\s+', ' ', ln.strip())[:70]
            break
    if arms:
        return '/'.join(arms) + f'(rc{rc})'
    if rc != 0:
        return err or f'ERR:rc{rc}'
    return 'NONE'

if __name__ == '__main__':
    tools = sys.argv[1].split(',')
    flt = sys.argv[2] if len(sys.argv) > 2 else ''
    meta = json.load(open(D + '/meta.json'))
    ids = [c for c in meta if c.startswith(flt) or (flt and re.fullmatch(flt, c))]
    jobs = [(t, c) for t in tools for c in ids]
    with ThreadPoolExecutor(4) as ex:
        list(ex.map(lambda j: run(*j), jobs))
    print('done', len(jobs))
