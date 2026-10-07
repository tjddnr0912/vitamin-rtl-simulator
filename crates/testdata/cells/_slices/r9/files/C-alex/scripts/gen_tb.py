#!/usr/bin/env python3
"""gen_tb.py <rtl_dir> <module> <params_json_or_-> <outdir> <tag> <ncycles> <files...>

Pass 1: iverilog width probe (VCD header of the DUT instantiated with the params).
Pass 2: write <outdir>/tb_<tag>.v: clocks, reset, xorshift64 stimulus on every input at the
main clock's negedge (NBA), and a $display of every output at the same negedge (before the
NBAs land) -> a cycle-resolution text trace.
"""
import re, sys, os, json, subprocess

rtl, mod, pj, out, tag, ncyc = sys.argv[1:7]
files = sys.argv[7:]
params = {} if pj == '-' else json.loads(pj)
ncyc = int(ncyc)
os.makedirs(out, exist_ok=True)

src = None
for f in files:
    t = open(f).read()
    t2 = re.sub(r'/\*.*?\*/', '', t, flags=re.S)
    t2 = re.sub(r'//[^\n]*', '', t2)
    m = re.search(r'^\s*module\s+' + mod + r'\b(.*?)\)\s*;', t2, flags=re.S | re.M)
    if m:
        src = m.group(1)
        break
assert src, 'module not found'
# ports in source order with direction
ports = []
for pm in re.finditer(r'\b(input|output|inout)\s+(?:wire|reg|logic)?\s*(?:signed\s*)?(?:\[[^\]]*\]\s*)?(\w+)', src):
    ports.append((pm.group(1), pm.group(2)))

def pstr(p):
    if not p:
        return ''
    return '#(' + ', '.join('.%s(%s)' % (k, v) for k, v in p.items()) + ')'

# pass 1: widths from iverilog VCD
wv = os.path.join(out, 'w_%s.v' % tag)
vcd = os.path.join(out, 'w_%s.vcd' % tag)
open(wv, 'w').write('`timescale 1ns/1ps\nmodule tbw; %s %s u(); initial begin $dumpfile("%s"); $dumpvars(1, u); #1 $finish; end endmodule\n' % (mod, pstr(params), vcd))
vvp = os.path.join(out, 'w_%s.vvp' % tag)
WD = ['perl', '/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/c40e1dc3-96e1-4997-a40d-90a56b79d219/scratchpad/s2-review/sound/p3/wd.pl', '300', '6000000']
lg = os.path.join(out, 'w_%s.log' % tag)
r = subprocess.run(WD + [lg, 'iverilog', '-g2012', '-s', 'tbw', '-o', vvp, wv] + files, capture_output=True, text=True)
if 'rc=0 sig=0' not in r.stdout:
    print('WIDTHPROBE_IVERILOG_FAIL', r.stdout, open(lg).read()[-3000:])
    sys.exit(2)
r = subprocess.run(WD + [lg, 'vvp', '-n', vvp], capture_output=True, text=True)
os.remove(lg)
widths = {}
depth = 0
for line in open(vcd):
    if line.startswith('$scope'):
        depth += 1
    elif line.startswith('$upscope'):
        depth -= 1
    elif line.startswith('$var') and depth == 2:
        tk = line.split()
        widths[tk[4]] = int(tk[2])
for f in (wv, vcd, vvp):
    os.remove(f)

ins = [(d, n) for d, n in ports if d == 'input']
outs = [(d, n) for d, n in ports if d != 'input']
clocks = [n for d, n in ins if re.search(r'(^|_)clk($|_)', n)]
resets = [n for d, n in ins if re.search(r'(^|_)rst($|_)', n)]
data_in = [n for d, n in ins if n not in clocks and n not in resets]
main = 'clk' if 'clk' in clocks else (clocks[0] if clocks else None)

L = ['`timescale 1ns/1ps', 'module tb;']
half = [5, 7, 11, 13]
for i, c in enumerate(clocks):
    L.append('reg %s = 1\'b0; always #%d %s = ~%s;' % (c, half[i], c, c))
if not main:
    L.append('reg tb_clk = 1\'b0; always #5 tb_clk = ~tb_clk;')
    main = 'tb_clk'
for n in resets:
    L.append('reg %s = 1\'b1;' % n)
for n in data_in:
    w = widths[n]
    L.append('reg [%d:0] %s = %d\'d0;' % (w - 1, n, w))
for d, n in outs:
    w = widths[n]
    L.append('wire [%d:0] %s;' % (w - 1, n))
L.append('%s %s dut(%s);' % (mod, pstr(params), ', '.join('.%s(%s)' % (n, n) for d, n in ports)))
L.append('reg [63:0] tb_s = 64\'h0123456789abcdef; integer tb_cyc = 0;')
L.append('function [63:0] tb_xs; input [63:0] x; reg [63:0] t; begin t = x ^ (x << 13); t = t ^ (t >> 7); t = t ^ (t << 17); tb_xs = t; end endfunction')
if os.environ.get('KICK') == '1':
    # wake every always @* that reads an input before the first clock edge (t=1 < first posedge t=5)
    L.append('initial begin #1; %s end' % ' '.join('%s = ~%s;' % (n, n) for n in data_in))
L.append('always @(negedge %s) begin' % main)
fmt = ' '.join(['%0d'] + ['%h'] * len(outs))
L.append('  $display("%s", tb_cyc%s);' % (fmt, ''.join(', ' + n for d, n in outs)))
L.append('  tb_cyc = tb_cyc + 1;')
for n in resets:
    L.append('  %s <= (tb_cyc < 8);' % n)
for n in data_in:
    w = widths[n]
    k = (w + 63) // 64
    L.append('  tb_s = tb_xs(tb_s); %s <= {%d{tb_s}};' % (n, k))
L.append('  if (tb_cyc == %d) $finish;' % ncyc)
L.append('end')
L.append('endmodule')
open(os.path.join(out, 'tb_%s.v' % tag), 'w').write('\n'.join(L) + '\n')
print('OK ports=%d in=%d out=%d clocks=%s' % (len(ports), len(data_in), len(outs), clocks))
