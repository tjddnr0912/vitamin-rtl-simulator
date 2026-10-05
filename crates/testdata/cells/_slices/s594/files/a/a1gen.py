#!/usr/bin/env python3
# a1gen.py <outdir>: §4.5.594 plan audit cells.
#  A*  tier-3 positions (replication count, +: width, packed bound, repeat) x {pure, 🆕 AC call, 🆕 AE call, R1 call count}
#  B*  constant-function reduction over a local (🆕 AE) + generate/bound consumers
#  C*  delay positions (CA integer, CA/procedural time-literal, net decl)
#  D*  interface (and module twin) override sign channel, incl. a 128-bit declared target
#  E*  param_init_kept_loud with a >=32-bit width
import os, sys
out = sys.argv[1]; os.makedirs(out, exist_ok=True)
cells = {}
X8  = "localparam logic signed [7:0] X = -4;"
N   = "localparam int N = 2;"
ARR = "localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};"
ASD = "localparam logic signed [7:0] AS [0:1] = '{-8'sd4, 8'sd2};"
C1  = "localparam bit C = 1;"
FCASE = "function automatic int fcase(input int a); case (a) 2: fcase = 3; default: fcase = 1; endcase endfunction"
FL    = "function automatic logic [7:0] fl(input int a); logic [7:0] r; fl = r + a; endfunction"
F2    = "function automatic int f2(input int a); return a; endfunction"
FUNCS = [FCASE, FL, F2]

def mod(name, decls, body, extra="", wd=40):
    txt = " ".join(body) + " " + " ".join(x for x in decls if not x.startswith(("localparam", "function")))
    full = " ".join(body) + " " + " ".join(x for x in decls if not x.startswith("localparam")) + " " + extra
    def need(x):
        if x == ARR: return "A[" in txt
        if x == ASD: return "AS[" in txt
        if x == X8: return "X" in txt.replace("XOR", "")
        if x == N: return "N{" in full or "N =" in full
        if x == C1: return "C ?" in txt
        if x == FCASE: return "fcase(" in txt
        if x == FL: return "fl(" in txt
        if x == F2: return "f2(" in txt
        return True
    decls = [x for x in decls if need(x)]
    s = "`timescale 1ns/1ns\n" + extra + "module t;\n"
    for d in decls + body:
        s += "  " + d + "\n"
    s += f"  initial #{wd} $finish;\nendmodule\n"
    cells[name] = s

# ---------- A: tier-3 positions ----------
counts = {
  'R0':  "{N{1'b0}} + 2'd3",
  'L0':  "2'b00 + 2'd3",
  'E0':  "(A[1] - 8'd2) + 2'd3",
  'RAC': "{N{1'b0}} + fcase(2)",
  'LAC': "2'b00 + fcase(2)",
  'EAC': "(A[1] - 8'd2) + fcase(2)",
  'RAE': "{N{1'b0}} + fl(2)",
  'LAE': "2'b00 + fl(2)",
  'EAE': "(A[1] - 8'd2) + fl(2)",
  'R1c': "{f2(2){1'b0}} + 2'd3",
}
for cn, CE in counts.items():
    d = [N, ARR] + FUNCS
    mod(f"Arep_{cn}", d, [f'initial #1 $display("rep=%h", {{({CE}){{4\'hF}}}});'])
    mod(f"Apsw_{cn}", d + ["logic [15:0] v = 16'hABCD;"], [f'initial #1 $display("pw=%h", v[0 +: ({CE})]);'])
    mod(f"Arng_{cn}", d + [f"logic [({CE}):0] v;"], ['initial #1 $display("rb=%0d", $bits(v));'])
    mod(f"Arpt_{cn}", d + ["int k;"], [f'initial begin k = 0; repeat ({CE}) k = k + 1; #1 $display("k=%0d", k); end'])

# ---------- B: constant-function reduction over a local ----------
FR_AE_R = "function automatic logic fr(input int a); logic [3:0] r; fr = |{N{r}}; endfunction"
FR_AE_L = "function automatic logic fr(input int a); logic [3:0] r; fr = |{2{r}}; endfunction"
FR_OK_R = "function automatic logic fr(input logic [3:0] a); fr = |{N{a}}; endfunction"
FR_OK_L = "function automatic logic fr(input logic [3:0] a); fr = |{2{a}}; endfunction"
FA_OK_R = "function automatic logic fr(input logic [3:0] a); fr = &{N{a}}; endfunction"
FX_OK_R = "function automatic logic fr(input logic [3:0] a); fr = ^{N{a}, 1'b1}; endfunction"
for fn, F, arg in (('aeR', FR_AE_R, "2"), ('aeL', FR_AE_L, "2"), ('okR', FR_OK_R, "4'b0010"), ('okL', FR_OK_L, "4'b0010"),
                   ('andR', FA_OK_R, "4'hF"), ('xorR', FX_OK_R, "4'h1")):
    d = [N, F]
    mod(f"Bred_{fn}_lp", d, [f"localparam L = fr({arg});", 'initial #1 $display("L=%b", L);'])
    mod(f"Bred_{fn}_gi", d, [f'if (fr({arg})) begin : gt initial #1 $display("GI=then"); end else begin : ge initial #1 $display("GI=else"); end'])
    mod(f"Bred_{fn}_rb", d, [f"logic [fr({arg}) + 2:0] v;", 'initial #1 $display("vb=%0d", $bits(v));'])
    mod(f"Bred_{fn}_rt", d + ["logic q;"], [f'initial begin q = fr({arg}); #1 $display("RT=%b", q); end'])

# ---------- C: delay positions ----------
MON = 'always @(w) $display("w=%b t=%0t", w, $time);'
cdel = {
  'R5':  "(X + {N{1'b0}}) / 8'd50",
  'L5':  "(X + 2'b00) / 8'd50",
  'E5':  "(X | A[1]) / 8'd50",
  'S5':  "(AS[0] + 8'd0) / 8'd50",
  'R2':  "(X + {N{1'b0}}) - 8'd250",
  'E2':  "(X | A[1]) - 8'd252",
  'RAE': "{N{1'b0}} + fl(2)",
  'LAE': "2'b00 + fl(2)",
}
for dn, DE in cdel.items():
    d = [X8, N, ARR, ASD] + FUNCS
    mod(f"Cca_{dn}", d + ["wire w;", f"assign #({DE}) w = 1'b1;"], [MON])
mod("Cnet_R5", [X8, N] + [f"wire #((X + {{N{{1'b0}}}}) / 8'd50) w = 1'b1;"], [MON])
mod("Cnet_L5", [X8, N] + ["wire #((X + 2'b00) / 8'd50) w = 1'b1;"], [MON])
# time-literal lanes (delay_units_in_scope): a whole replication / element operand of a real product
tl = {
  'AS': "AS[0] * 1ns",
  'X':  "X * 1ns",
  'R':  "(X + {N{1'b0}}) * 1ns",
  'L':  "(X + 2'b00) * 1ns",
  'E':  "(X | A[1]) * 1ns",
  'A1': "A[1] * 1ns",
  'AS1': "AS[1] * 1ns",
}
for tn, TE in tl.items():
    d = [X8, N, ARR, ASD]
    mod(f"Cpt_{tn}", d, [f'initial begin #({TE}); $display("fired t=%0t", $time); end'], wd=400)
    mod(f"Cct_{tn}", d + ["wire w;", f"assign #({TE}) w = 1'b1;"], [MON], wd=400)
mod("Cca_AS", [X8, N, ARR, ASD] + ["wire w;", "assign #(AS[0]) w = 1'b1;"], [MON], wd=400)
mod("Cca_X",  [X8, N, ARR, ASD] + ["wire w;", "assign #(X) w = 1'b1;"], [MON], wd=400)

# ---------- D: override sign channel (interface + module twin), incl. 128-bit target ----------
IFU = "interface ifc #(parameter P = 1);\n  initial #1 $display(\"P=%0d lt0=%0d B=%0d\", P, P < 0, $bits(P));\nendinterface\n"
IFT = "interface ifc #(parameter logic [15:0] P = 0);\n  initial #1 $display(\"P=%0d\", P);\nendinterface\n"
IFI = "interface ifc #(parameter int P = 0);\n  initial #1 $display(\"P=%0d\", P);\nendinterface\n"
IFW = "interface ifc #(parameter logic [127:0] P = 0);\n  initial #1 $display(\"P=%h\", P);\nendinterface\n"
MW  = "module m #(parameter logic [127:0] P = 0);\n  initial #1 $display(\"P=%h\", P);\nendmodule\n"
MU  = "module m #(parameter P = 1);\n  initial #1 $display(\"P=%0d lt0=%0d B=%0d\", P, P < 0, $bits(P));\nendmodule\n"
oexp = {'s': "AS[0]", 'u': "A[0]", 'se': "AS[0] + 8'sd0", 'st': "C ? AS[0] : AS[1]", 'sm': "AS[0] | A[1]",
        'R': "X + {N{1'b0}}", 'L': "X + 2'b00", 'Rs': "X + {N{1'sb0}}"}
dd = [X8, N, ARR, ASD, C1]
for en, E in oexp.items():
    mod(f"Diu_{en}", dd, [f"ifc #(.P({E})) u();"], IFU)
    mod(f"Diw_{en}", dd, [f"ifc #(.P({E})) u();"], IFW)
    mod(f"Dmw_{en}", dd, [f"m #(.P({E})) u();"], MW)
    if en in ('s', 'se', 'st', 'R'):
        mod(f"Dit_{en}", dd, [f"ifc #(.P({E})) u();"], IFT)
        mod(f"Dii_{en}", dd, [f"ifc #(.P({E})) u();"], IFI)
        mod(f"Dipu_{en}", dd, [f"ifc #({E}) u();"], IFU)
        mod(f"Dipw_{en}", dd, [f"ifc #({E}) u();"], IFW)
        mod(f"Dmu_{en}", dd, [f"m #(.P({E})) u();"], MU)

# ---------- E: param_init_kept_loud at >= 32 bits ----------
SUBK = "module m #(parameter int N = 40, parameter P = (|{N{1'b1}}) + {N{1'b0}});\n  initial #1 $display(\"P=%0d B=%0d\", P, $bits(P));\nendmodule\n"
SUBKL = "module m #(parameter int N = 40, parameter P = (|{40{1'b1}}) + {40{1'b0}});\n  initial #1 $display(\"P=%0d B=%0d\", P, $bits(P));\nendmodule\n"
for sn, sub in (('R', SUBK), ('L', SUBKL)):
    mod(f"Ekl_{sn}_d", [], ["m u();"], sub)
    mod(f"Ekl_{sn}_o5", [], ["m #(.P(-5)) u();"], sub)
    mod(f"Ekl_{sn}_o8", [], ["m #(.P(8'd5)) u();"], sub)

for n, s in cells.items():
    open(os.path.join(out, n + ".sv"), "w").write(s)
print(len(cells))
