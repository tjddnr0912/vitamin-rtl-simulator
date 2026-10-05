#!/usr/bin/env python3
import os, re, subprocess, sys, json
from concurrent.futures import ThreadPoolExecutor
H = os.path.dirname(os.path.abspath(__file__)); D = H + '/cells'
SP = '/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad'
SV2V = SP + '/s580/sv2v/sv2v-macOS/sv2v'
VITA = {'pre': SP + '/s581/pre/vita', 'p3': SP + '/s580/post3/vita', 'post': SP + '/s581/post/vita', 'p2': SP + '/s581/post2/vita', 'p3': SP + '/s581/post3/vita', 'postT': SP + '/s581/postT/vita'}
def sh(cmd, t=300):
    try:
        r = subprocess.run(cmd, shell=True, capture_output=True, text=True, cwd=D, timeout=t); return r.stdout + r.stderr, r.returncode
    except subprocess.TimeoutExpired: return 'TIMEOUT', -9
def run(tool, c):
    f = c + '.sv'
    if tool == 'iv': o, rc = sh(f'iverilog -g2012 -o {c}.vvp {f} && vvp -n {c}.vvp')
    elif tool == 'sv': o, rc = sh(f'{SV2V} {f} > {c}.sv2v.v && iverilog -g2012 -o {c}.s.vvp {c}.sv2v.v && vvp -n {c}.s.vvp')
    elif tool == 'vl': o, rc = sh(f'rm -rf o_{c}; verilator --binary --timing -Wno-fatal -Wno-lint -Wno-style --top-module t {f} -Mdir o_{c} > {c}.vlb 2>&1; r=$?; grep -m1 "%Error" {c}.vlb; [ $r = 0 ] && o_{c}/Vt', 600)
    else: o, rc = sh(f'{VITA[tool]} {f}')
    open(f'{D}/{c}.{tool}.txt', 'w').write(o + f'\nrc={rc}\n')
def verdict(tool, c):
    p = f'{D}/{c}.{tool}.txt'
    if not os.path.exists(p): return '?'
    t = open(p).read(); rc = int(re.search(r'\nrc=(-?\d+)\n$', t).group(1))
    ls = [l.strip() for l in t.split('\n') if re.match(r'^[A-Z][A-Za-z0-9_]*\s', l) and not l.startswith(('VCD', 'VITA')) and '$finish' not in l and not l.startswith('- ')]
    ls = [l for l in ls if not re.search(r'(error|warning|sorry|Error)', l)]
    if rc != 0 or not ls:
        m = re.search(r'(error\[VITA-\S+\]|%Error[^\n]{0,50}|sorry[^\n]{0,40}|error:[^\n]{0,50})', t)
        return 'ERR:' + (m.group(1)[:50] if m else f'rc{rc}') if rc != 0 else 'NONE'
    return ' | '.join(sorted(ls))
if __name__ == '__main__':
    tools = sys.argv[1].split(','); cs = json.load(open(D + '/list.json'))
    if len(sys.argv) > 2: cs = [c for c in cs if re.match(sys.argv[2], c)]
    with ThreadPoolExecutor(4) as ex: list(ex.map(lambda j: run(*j), [(t, c) for t in tools for c in cs]))
    print('done')
