#!/usr/bin/env python3
"""lens:differential round 3 — generate variants of 12 hand-written designs, run PRE / POST3b / iverilog / sv2v->iverilog."""
import os, re, subprocess, sys
from concurrent.futures import ThreadPoolExecutor

SCR = "/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s581"
L = SCR + "/lens_diff3"
OUT = L + "/v3"
SV2V = SCR + "/../s580/sv2v/sv2v-macOS/sv2v"
BINS = [("pre", SCR + "/pre/vita"), ("post3b", SCR + "/post3b/vita")]
os.makedirs(OUT, exist_ok=True)

DECL = {
    "P40": "localparam logic [39:0] P40 = 40'h10_0000_000C;",
    "P40F": "localparam logic [39:0] P40F = 40'h0C;",
    "P68H": "localparam logic [67:0] P68H = 68'h1_0000_0000_0000_000C;",
    "P68F": "localparam logic [67:0] P68F = 68'hC;",
    "P64": "localparam logic [63:0] P64 = 64'hC;",
    "P64H": "localparam logic [63:0] P64H = 64'h8000_0000_0000_000C;",
    "P65": "localparam logic [64:0] P65 = 65'hC;",
    "P65H": "localparam logic [64:0] P65H = 65'h0_8000_0000_0000_000C;",
    "S64N": "localparam logic signed [63:0] S64N = -64'sd4;",
    "S64P": "localparam logic signed [63:0] S64P = 64'sd12;",
    "S65N": "localparam logic signed [64:0] S65N = -65'sd4;",
    "S65P": "localparam logic signed [64:0] S65P = 65'sd12;",
    "N2": "localparam int N2 = 2;",
    "E0": "typedef enum logic [39:0] {E0 = 40'h10_0000_000C, E1 = 40'h0C} et40;",
    "E1": "typedef enum logic [39:0] {E0 = 40'h10_0000_000C, E1 = 40'h0C} et40;",
    "ES": "typedef enum logic signed [3:0] {ES = -4'sd4, ET = 4'sd3} est4;",
    "t40": "typedef logic [39:0] t40;",
    "PT": "typedef logic [39:0] t40;\nlocalparam t40 PT = 40'h10_0000_000C;",
    "PTF": "typedef logic [39:0] t40;\nlocalparam t40 PTF = 40'h0C;",
    "SS": "typedef struct packed {logic [39:0] a; logic [3:0] b;} st;\nlocalparam st SS = '{a: 40'h10_0000_000C, b: 4'hC};",
    "PA": "localparam logic [39:0] PA [2] = '{40'h10_0000_000C, 40'h0};",
    "PP": "localparam logic [1:0][39:0] PP = {40'h10_0000_000C, 40'h1};",
    "f40": "function automatic logic [39:0] f40(input int a); return 40'h10_0000_0000 + a; endfunction",
    "ft40": "typedef logic [39:0] t40;\nfunction automatic t40 ft40(input int a); return 40'h10_0000_0000 + a; endfunction",
    "finc": "function automatic int finc(input int a); return a + 5; endfunction",
    "fs64": "function automatic logic signed [63:0] fs64(input int a); return -a; endfunction",
    "f65": "function automatic logic [64:0] f65(input int a); return a; endfunction",
    "PX": "localparam logic [3:0] PX = 4'b1x00;",
}


def decls_for(text, extra=""):
    names = set(re.findall(r"(?<!['\w])[A-Za-z_]\w*", text))
    lines, seen = [], set()
    for k, v in DECL.items():
        if k in names:
            for ln in v.split("\n"):
                if ln not in seen:
                    seen.add(ln)
                    lines.append(ln)
    out = ([extra] if extra else []) + lines
    return "\n".join("  " + x for x in out)


LP = 'module top;\n@D@\n  localparam R = (@E@);\n  initial $display("R=%0d", R);\n  initial #100 $finish;\nendmodule\n'
GI = ('module top;\n@D@\n  if (@E@) begin : g\n    initial $display("GI=then");\n  end else begin : g\n'
      '    initial $display("GI=else");\n  end\n  initial #100 $finish;\nendmodule\n')
rows = []


def emit(vid, design, text, exp):
    p = f"{OUT}/{vid}.sv"
    with open(p, "w") as f:
        f.write(text)
    rows.append((vid, design, p, exp))


def lhs(design, vid, expr, e, pre="", hdr=None, extra="", cons=("lp", "gi")):
    for c in cons:
        t = (LP if c == "lp" else GI).replace("@D@", decls_for(expr, extra)).replace("@E@", expr)
        if hdr:
            t = t.replace("module top;", hdr, 1)
        exp = f"R={e}" if c == "lp" else {"1": "GI=then", "0": "GI=else", "x": "GI=else"}.get(e, "?")
        emit(f"{vid}_{c}", design, (pre + "\n" + t) if pre else t, exp)


def body(design, vid, b, exp, pre=""):
    t = "module top;\n" + decls_for(b) + "\n  " + b + "\n  initial #100 $finish;\nendmodule\n"
    emit(vid, design, (pre + "\n" + t) if pre else t, exp)


# ---- D1 system-call left operands
DECL.update({
 "PU": "localparam int unsigned PU = 32'hFFFF_FFFC;", "PLU": "localparam longint unsigned PLU = 64'hFFFF_FFFF_FFFF_FFFC;",
 "PBU": "localparam byte unsigned PBU = 8'hFC;", "PSU": "localparam shortint unsigned PSU = 16'hFFFC;",
 "PIU": "localparam integer unsigned PIU = 32'hFFFF_FFFC;",
 "PUT": "typedef int unsigned u32_t;\nlocalparam u32_t PUT = 32'hFFFF_FFFC;",
 "t64s": "typedef logic signed [63:0] t64s;", "t64u": "typedef logic [63:0] t64u;", "t40s": "typedef logic signed [39:0] t40s;",
 "SG": "typedef struct packed {logic signed [63:0] a; logic [3:0] b;} sts;\nlocalparam sts SG = '{a: -64'sd4, b: 4'h0};",
 "SK": "typedef struct packed signed {logic [59:0] a; logic [3:0] b;} spk;\nlocalparam spk SK = '{a: 60'hFFF_FFFF_FFFF_FFFF, b: 4'hC};",
 "PI": "localparam integer PI = -4;", "PBS": "localparam bit signed [63:0] PBS = -64'sd4;",
 "PLS": "localparam longint PLS = -64'sd4;",
})
for vid, ex, e in [
 ("S01", "PU ==? 4'sb1?00", "0"), ("S02", "PLU ==? 4'sb1?00", "0"), ("S03", "PBU ==? 4'sb1?00", "0"),
 ("S04", "PSU ==? 4'sb1?00", "0"), ("S05", "PIU ==? 4'sb1?00", "0"), ("S06", "PUT ==? 4'sb1?00", "0"),
 ("S07", "t64s'(64'hFFFF_FFFF_FFFF_FFFC) ==? 4'sb1?00", "1"), ("S08", "t64u'(-64'sd4) ==? 4'sb1?00", "0"),
 ("S09", "t40s'(40'hFF_FFFF_FFFC) ==? 4'sb1?00", "1"), ("S10", "t64u'(-64'sd4) ==? 4'b1?00", "0"),
 ("S11", "SG.a ==? 4'sb1?00", "1"), ("S12", "SK ==? 4'sb1?00", "1"),
 ("S13", "$bits(P40) ==? 6'sb10_1?00", "?"), ("S14", "$clog2(P40) ==? 6'sb10_0?01", "?"),
 ("S16", "PI ==? 4'sb1?00", "1"), ("S18", "PLU ==? 4'b1?00", "0"), ("S19", "PBS ==? 4'sb1?00", "1"),
 ("S21", "unsigned'(-64'sd4) ==? 4'sb1?00", "0"), ("S22", "signed'(64'hFFFF_FFFF_FFFF_FFFC) ==? 4'sb1?00", "1"),
 ("S23", "PLS ==? 4'sb1?00", "1"), ("S24", "PLU !=? 4'sb1?00", "1"), ("S25", "PU ==? 'sbx100", "1"),
]:
    lhs("D13" if False else "D02", vid, ex, e)
TAG = re.compile(r"^(R|GI|G)=")


def norm(out):
    keep = []
    for ln in out.splitlines():
        if TAG.match(ln):
            keep.append(ln.strip())
        m = re.search(r"(?:error|fatal)\[(VITA-E\d+)\]", ln)
        if m:
            keep.append(m.group(1))
    return ";".join(sorted(keep)) or "<none>"


def run_vita(b, p):
    try:
        r = subprocess.run([b, os.path.basename(p)], cwd=os.path.dirname(p), capture_output=True, text=True, timeout=60)
    except subprocess.TimeoutExpired:
        return "TIMEOUT"
    s = norm(r.stdout + r.stderr)
    return s if r.returncode in (0, 1) else f"{s}(rc{r.returncode})"


def run_iv(p, tag):
    vvp = p.rsplit(".", 1)[0] + f".{tag}.vvp"
    r = subprocess.run(["iverilog", "-g2012", "-o", vvp, p], capture_output=True, text=True, timeout=60)
    if r.returncode != 0:
        with open(p + f".{tag}.err", "w") as f:
            f.write(r.stdout + r.stderr)
        return "IVERR"
    r = subprocess.run(["vvp", "-n", vvp], capture_output=True, text=True, timeout=60)
    return ";".join(sorted(ln.strip() for ln in r.stdout.splitlines() if TAG.match(ln))) or "<none>"


def run_sv(p):
    v = p.rsplit(".", 1)[0] + ".s2v.v"
    r = subprocess.run([SV2V, p], capture_output=True, text=True, timeout=60)
    if r.returncode != 0:
        with open(p + ".sv2v.err", "w") as f:
            f.write(r.stderr)
        return "SV2VERR"
    with open(v, "w") as f:
        f.write(r.stdout)
    return run_iv(v, "s")


def one(row):
    vid, d, p, exp = row
    res = [run_vita(b, p) for _, b in BINS]
    return (vid, d, exp, res[0], res[1], run_iv(p, "i"), run_sv(p))


with ThreadPoolExecutor(6) as ex:
    res = list(ex.map(one, rows))
with open(L + "/r3c.tsv", "w") as f:
    f.write("id\tdesign\texp\tpre\tpost3b\tiverilog\tsv2v\n")
    for r in res:
        f.write("\t".join(r) + "\n")
designs = sorted({r[1] for r in res})
moved = [r for r in res if r[3] != r[4]]
print(f"designs={len(designs)} variants={len(res)} moved(PRE!=POST3b)={len(moved)}")
for d in designs:
    rr = [r for r in res if r[1] == d]
    print(f"  {d}: n={len(rr)} moved={sum(1 for r in rr if r[3] != r[4])}")
print("== MOVED rows: id | exp | PRE | POST3b | iverilog | sv2v")
for r in moved:
    print(" | ".join([r[0], r[2], r[3], r[4], r[5], r[6]]))
print("== NOT moved, POST3b differs from an agreeing oracle pair (pre-existing candidates)")
for r in res:
    if r[3] == r[4] and r[5] == r[6] and r[5] not in ("IVERR", "<none>") and r[4] != r[5]:
        print(" | ".join([r[0], r[2], r[3], r[4], r[5], r[6]]))
