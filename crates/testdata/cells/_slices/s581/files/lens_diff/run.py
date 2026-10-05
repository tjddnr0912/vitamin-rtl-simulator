#!/usr/bin/env python3
# run.py TOOLS REGEX  -- run cells p/<id>.sv through tools; raw output cached in p/<id>.<tool>.txt
# optional p/<id>.G : lines "NAME=VALUE" = top-param overrides (vita -G, iverilog -Ptop., verilator -G; sv2v: n/a)
import os, re, subprocess, sys, shlex
from concurrent.futures import ThreadPoolExecutor
L = os.path.dirname(os.path.abspath(__file__)); D = L + '/p'
SCR = os.path.dirname(L); SV2V = SCR + '/../s580/sv2v/sv2v-macOS/sv2v'
VITA = {'pre': SCR + '/pre/vita', 'pt': SCR + '/postT/vita', 'post': SCR + '/post/vita', 'ref': SCR + '/../s580/post3/vita'}
ENV = dict(os.environ, DEVELOPER_DIR='/Library/Developer/CommandLineTools')
def sh(cmd, timeout=600):
    try:
        r = subprocess.run(cmd, shell=True, capture_output=True, text=True, cwd=D, timeout=timeout, env=ENV)
        return r.stdout + r.stderr, r.returncode
    except subprocess.TimeoutExpired:
        return 'TIMEOUT', -9
def gargs(cid):
    p = f'{D}/{cid}.G'
    if not os.path.exists(p): return []
    return [l.strip().split('=', 1) for l in open(p) if '=' in l]
def run(tool, cid):
    f = f'{cid}.sv'; G = gargs(cid)
    if tool == 'iv':
        g = ' '.join(shlex.quote(f"-Ptop.{n}={v}") for n, v in G)
        out, rc = sh(f'iverilog -g2012 {g} -o {cid}.vvp {f} && vvp -n {cid}.vvp')
    elif tool == 'sv':
        if G: out, rc = 'NA(-G)', 0
        else: out, rc = sh(f'{SV2V} {f} > {cid}.sv2v.v && iverilog -g2012 -o {cid}.s.vvp {cid}.sv2v.v && vvp -n {cid}.s.vvp')
    elif tool == 'vl':
        g = ' '.join(shlex.quote(f"-G{n}={v}") for n, v in G)
        out, rc = sh(f'rm -rf obj_{cid}; verilator --binary --timing -Wno-fatal -Wno-lint -Wno-style {g} --top-module top {f} -Mdir obj_{cid} > {cid}.vlb.txt 2>&1; r=$?; grep -E "%Error" {cid}.vlb.txt | head -3; [ $r = 0 ] && obj_{cid}/Vtop; e=$?; rm -rf obj_{cid}; exit $e')
    else:
        g = ' '.join('-G ' + shlex.quote(f"{n}={v}") for n, v in G)
        out, rc = sh(f'{VITA[tool]} {g} {f}')
    open(f'{D}/{cid}.{tool}.txt', 'w').write(out + f'\nrc={rc}\n')
def verdict(tool, cid):
    p = f'{D}/{cid}.{tool}.txt'
    if not os.path.exists(p): return '?'
    t = open(p).read()
    if t.startswith('NA('): return 'NA'
    m = re.search(r'\nrc=(-?\d+)\n$', t); rc = int(m.group(1)) if m else 99
    lines = sorted(l.strip() for l in t.split('\n') if l.startswith('@'))
    err = None
    for ln in t.split('\n'):
        mm = re.match(r'^(?:(\S+):(\d+):(\d+): )?(error|fatal)\[(VITA-\S+)\] (\S+): (.*)$', ln)
        if mm: err = f'ERR:{mm.group(5)}'; break
        if re.search(r'%Error|error:|syntax error|sorry|Unable|unsupported|TIMEOUT|panic', ln, re.I):
            err = 'ERR:' + re.sub(r'\s+', ' ', ln.strip())[:60]; break
    s = ' | '.join(l[1:].strip() for l in lines)
    if rc != 0: s = (s + ' ' if s else '') + (err or f'ERR:rc{rc}') + f'(rc{rc})'
    return s or 'NONE'
if __name__ == '__main__':
    tools = sys.argv[1].split(','); rx = sys.argv[2] if len(sys.argv) > 2 else '.*'
    ids = sorted(f[:-3] for f in os.listdir(D) if f.endswith('.sv') and not f.endswith('.sv2v.v') and re.fullmatch(rx, f[:-3]))
    if tools != ['show']:
        with ThreadPoolExecutor(4) as ex: list(ex.map(lambda j: run(*j), [(t, c) for t in tools for c in ids]))
    T = ['iv', 'sv', 'vl', 'pre', 'pt', 'post']
    for c in ids:
        v = {t: verdict(t, c) for t in T}
        flag = '' if len(set(v[t] for t in ('pre', 'pt', 'post'))) == 1 else '  <<VITA-MOVES'
        orc = [v[t] for t in ('iv', 'sv', 'vl') if v[t] not in ('NA', '?') and not v[t].startswith('ERR')]
        if orc and v['post'] not in orc: flag += '  <<POST!=ORACLE'
        print(f'== {c}{flag}')
        for t in T: print(f'   {t:4} {v[t]}')
