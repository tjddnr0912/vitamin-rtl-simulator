#!/usr/bin/env python3
# random label sweep for generate-case. pass1: iverilog evaluates each label self-determined;
# pass2: R<k>.sv cells, each label tried against scrutinees built from its own value.
import random, os, sys, subprocess, re
P = os.path.dirname(os.path.abspath(__file__)) + '/p'
random.seed(int(sys.argv[2]) if len(sys.argv) > 2 else 581)
DECL = """  localparam logic [3:0] P4 = 4'hC;
  localparam logic signed [3:0] S4 = -4'sd3;
  localparam logic [7:0] P8 = 8'hF0;
  localparam logic signed [7:0] S8 = -8'sd100;
  localparam int I = -5;
  localparam int unsigned U32 = 32'hFFFF_FFF0;
  localparam logic [64:0] P65 = {1'b1, 64'h5};
  localparam logic signed [64:0] S65 = -65'sd7;
  localparam longint L64 = -9;
  parameter UN = 4'hA;
  typedef logic signed [5:0] s6_t;
  localparam int W6 = 6;
  localparam logic [127:0] P128 = {64'hFFFF_FFFF_FFFF_FFFF, 64'h0123_4567_89AB_CDEF};
"""
NAMES = ['P4', 'S4', 'P8', 'S8', 'I', 'U32', 'P65', 'S65', 'L64', 'UN', 'P128', 'W6']
def lit():
    w = random.choice([1, 2, 3, 4, 5, 8, 16, 31, 32, 33, 63, 64, 65])
    s = random.random() < 0.4
    v = random.getrandbits(w)
    r = random.random()
    if r < 0.15: return str(random.randint(0, 40))
    if r < 0.25: return f"(-{random.randint(1, 9)})"
    return f"{w}'{'s' if s else ''}h{v:x}"
def leaf():
    return random.choice(NAMES) if random.random() < 0.45 else lit()
def sized(d):
    w = random.choice([1, 3, 4, 8, 33, 65])
    return f"{w}'({expr(d)})" if random.random() < 0.5 else random.choice(['P4', 'S4', 'P8', 'S8', 'P65', 'S65', "4'hB", "8'sh9C", "33'h1_0000_0001"])
def expr(d):
    if d <= 0: return leaf()
    r = random.random()
    if r < 0.12: return f"({random.choice(['-', '~', '!', '&', '|', '^', '~&', '~|', '~^'])}{expr(d-1)})"
    if r < 0.22: return random.choice([f"signed'({expr(d-1)})", f"unsigned'({expr(d-1)})", f"s6_t'({expr(d-1)})", f"W6'({expr(d-1)})", f"{random.choice([1,3,6,33,65,70])}'({expr(d-1)})"])
    if r < 0.30: return random.choice([f"$countones({sized(d-1)})", f"$onehot({sized(d-1)})", f"$onehot0({sized(d-1)})", f"$clog2({expr(d-1)})", f"({expr(d-1)} ** {random.randint(0,3)})"])
    if r < 0.55: return f"({expr(d-1)} {random.choice(['+', '-', '*', '&', '|', '^', '<<', '>>', '>>>', '/', '%', '==', '<', '>=', '&&', '||'])} {expr(d-1)})"
    if r < 0.65: return f"{random.choice(['$signed', '$unsigned'])}({expr(d-1)})"
    if r < 0.75: return f"{{{sized(d-1)}, {sized(d-1)}}}"
    if r < 0.80: return f"{{2{{{sized(d-1)}}}}}"
    if r < 0.88: return f"({expr(d-1)} ? {expr(d-1)} : {expr(d-1)})"
    return leaf()
def fix_shift(e):  # keep shift amounts small: replace rhs of << >> >>> by small literal
    return re.sub(r'(<<|>>>|>>) ([^()]+?)\)', lambda m: f"{m.group(1)} {random.randint(0, 5)})", e)
N = int(sys.argv[1]); PFX = sys.argv[3] if len(sys.argv) > 3 else 'R'
labels = [fix_shift(expr(random.choice([1, 2, 2, 3]))) for _ in range(N)]
labels = [l for l in labels if '/ ' not in l or True]
# pass 1
src = "module top;\n" + DECL + "  initial begin\n" + ''.join(f'    $display("@E{k} %0d %b", $bits({l}), {l});\n' for k, l in enumerate(labels)) + "  end\nendmodule\n"
open(f'{P}/R_eval.sv', 'w').write(src)
r = subprocess.run(f'iverilog -g2012 -o R_eval.vvp R_eval.sv && vvp -n R_eval.vvp', shell=True, cwd=P, capture_output=True, text=True)
val = {}
for m in re.finditer(r'^@E(\d+) (\d+) ([01xz]+)$', r.stdout, re.M):
    val[int(m.group(1))] = (int(m.group(2)), m.group(3))
print('pass1', len(val), 'of', N, r.stderr[:300])
# pass 2: scrutinees from the value
cells, k = [], 0
for i, l in enumerate(labels):
    if i not in val: continue
    w, b = val[i]
    if 'x' in b or 'z' in b or w > 64: continue
    v = int(b, 2)
    sv = v - (1 << w) if b[0] == '1' else v
    scr = [f"{w}'h{v:x}", f"{w}'sh{v:x}", f"{min(w+3, 64)}'h{v:x}", f"64'h{(sv % (1 << 64)):x}"] + ([f"{sv}" if sv >= 0 else f"(-{-sv})"] if abs(sv) < 2**31 else []) + ([f"{v}"] if v < 2**31 else [])
    for s in scr:
        cells.append((s, l))
print('cells', len(cells))
per = 25
for f in range(0, len(cells), per):
    body = ''.join(f'  case ({s}) {l}: begin : h{j} initial $display("@{PFX}{f//per}_{j} hit"); end default: begin : d{j} initial $display("@{PFX}{f//per}_{j} def"); end endcase\n' for j, (s, l) in enumerate(cells[f:f+per]))
    open(f'{P}/{PFX}{f//per:02d}.sv', 'w').write("module top;\n" + DECL + body + "endmodule\n")
print('files', (len(cells) + per - 1) // per)
