#!/usr/bin/env python3
# gen2.py <outdir>: twins, lanes (self positions, tier-3 bounds/counts, fill/wide fences, override,
# kept-loud), element specifics. One display per file.
import os, sys
out = sys.argv[1]; os.makedirs(out, exist_ok=True)
cells = {}
X8 = "localparam logic signed [7:0] X = -4;"
N = "localparam int N = 2;"
ARR = "localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};"
C1 = "localparam bit C = 1;"
PKG = "package p;\n  localparam logic [7:0] PA [0:1] = '{8'hFC, 8'h02};\nendpackage\n"
def mod(name, decls, body, pkg=False, extra=""):
    s = "`timescale 1ns/1ns\n" + (PKG if pkg else "") + extra + "module t;\n"
    for d in decls + body: s += "  " + d + "\n"
    s += "  initial #20 $finish;\nendmodule\n"
    cells[name] = s
def cons(E, K, gt, w=8):
    return {
      'lp': ([f"localparam L = (({E}) == {K});"], 'initial #1 $display("L=%0d", L);'),
      'lg': ([f"localparam L = (({E}) > {gt});"], 'initial #1 $display("L=%0d", L);'),
      'lv': ([f"localparam L = {E};"], 'initial #1 $display("L=%0d B=%0d", L, $bits(L));'),
      'lt': ([f"localparam logic [{w*2-1}:0] L = {E};"], 'initial #1 $display("L=%0d", L);'),
      'gi': ([f"if (({E}) == {K}) begin : gt initial #1 $display(\"GI=then\"); end else begin : ge initial #1 $display(\"GI=else\"); end"], None),
      'gc': ([f"case ({E}) {K}: begin : gk initial #1 $display(\"GC=item\"); end default: begin : gd initial #1 $display(\"GC=def\"); end endcase"], None),
      'rb': ([f"logic [(({E}) == {K}) + 3:0] v;"], 'initial #1 $display("vb=%0d", $bits(v));'),
      'rt': ([], f'initial #1 $display("RT=%0d", (({E}) == {K}));'),
      'rv': ([], f'initial #1 $display("RV=%0d", {E});'),
    }
def addc(name, decls, E, K, gt, w=8, only=None, pkg=False):
    for c, (cd, disp) in cons(E, K, gt, w).items():
        if only and c not in only: continue
        mod(f"{name}_{c}", decls, cd + ([disp] if disp else []), pkg)
# A. literal twins of the None leaves (attribution of non-row-X consumers)
addc("Tl2_a", [X8, C1], "X + 2'b00", "8'hFC", "8'd100")
addc("Tl8_o", [X8, C1], "X | 8'h02", "8'hFE", "8'd100")
addc("Tl8_t", [X8, C1], "C ? X : 8'h02", "8'hFC", "8'd100")
for an, E in (('div', "X / 2'b11"), ('mod', "X % 2'b11"), ('shr', "(X + 2'b00) >> 1"), ('ediv', "X / 8'h02"), ('eshr', "(X | 8'h02) >> 1")):
    addc(f"Tar{an}", [X8], E, "8'h0", "8'd0", only=('lv', 'lt', 'rv'))
# B. self-determined positions
selfpos = {
 'cond': ("((8'hFF + 8'd1 + {Z0}) ? 5 : 7)",),
 'idx':  ("W[(X + {Z0}) - 8'd248]",),
 'shc':  ("16'h0001 << ((X + {Z0}) - 8'd250)",),
 'pow':  ("2 ** ((X + {Z0}) - 8'd250)",),
 'clog': ("$clog2(X + {Z0})",),
 'lnot': ("!(8'hFF + 8'd1 + {Z0})",),
 'red':  ("&(X + {Z0})",),
 'cast': ("16'(X + {Z0})",),
 'scast':("$unsigned(X + {Z0})",),
}
for zn, Z0, ex in (('R', "{N{1'b0}}", [N]), ('L', "2'b00", []), ('E', "(A[1] - 8'd2)", [ARR])):
    for sn, (tmpl,) in selfpos.items():
        E = tmpl.format(Z0=Z0)
        mod(f"S{sn}_{zn}", [X8, "localparam logic [15:0] W = 16'h00F0;"] + ex,
            [f"localparam L = {E};", 'initial #1 $display("L=%0d", L);'])
        mod(f"S{sn}_{zn}_rt", [X8, "localparam logic [15:0] W = 16'h00F0;"] + ex,
            [f'initial #1 $display("RT=%0d", {E});'])
    # delay position
    mod(f"Sdly_{zn}", [X8] + ex, ["logic w = 0;", f"assign #((X + {Z0}) - 8'd250) w = 1'b1;",
        "initial begin #1 $display(\"t1 w=%b\", w); #2 $display(\"t3 w=%b\", w); end"])
# C. tier-3 bound / count positions (loud->value audit)
for zn, Z, ex in (('R', "{N{1'b1}}", [N]), ('L', "2'b11", []), ('E', "A[1][1:0] | 2'b11", [ARR]), ('Ee', "A[1]", [ARR])):
    two = "2'd2" if zn != 'Ee' else "8'd255"
    E = f"({Z} + {two})"
    mod(f"Cbnd_{zn}", ex, [f"logic [{E}:0] b;", 'initial #1 $display("bb=%0d", $bits(b));'])
    mod(f"Crep_{zn}", ex, [f"localparam logic [31:0] R = {{{E}{{4'hA}}}};", 'initial #1 $display("R=%h", R);'])
    mod(f"Cps_{zn}", ex, ["logic [15:0] v = 16'hFFFF;", f'initial #1 $display("ps=%0d", $bits(v[0 +: {E}]));'])
    mod(f"Cdim_{zn}", ex, [f"logic d [{E}:0];", 'initial #1 $display("ds=%0d", $size(d));'])
    mod(f"Crpt_{zn}", ex, ["int cnt = 0;", f"initial begin repeat ({E}) cnt++; #1 $display(\"cnt=%0d\", cnt); end"])
    mod(f"Cgfr_{zn}", ex, [f"for (genvar g = 0; g < {E}; g = g + 1) begin : gl initial #1 $display(\"g=%0d\", g); end"])
mod("Cebn_E", [ARR], ["logic [A[1] - 8'd3:0] eb;", 'initial #1 $display("eb=%0d", $bits(eb));'])
mod("Cebn_L", [], ["logic [8'h02 - 8'd3:0] eb;", 'initial #1 $display("eb=%0d", $bits(eb));'])
# D. fill / >64-bit fences
mod("Dfw80t", [N], ["localparam logic [79:0] L = '1 ^ {N{40'h0}};", 'initial #1 $display("L=%h", L);'])
mod("Dfw80u", [N], ["localparam U = '1 ^ {N{40'h0}};", 'initial #1 $display("U=%h B=%0d", U, $bits(U));'])
mod("Dfw8t", [N], ["localparam logic [7:0] L = '1 ^ {N{1'b0}};", 'initial #1 $display("L=%h", L);'])
mod("Dfw2u", [N], ["localparam U = '1 ^ {N{1'b0}};", 'initial #1 $display("U=%h B=%0d", U, $bits(U));'])
mod("Dfw2uL", [], ["localparam U = '1 ^ {2{1'b0}};", 'initial #1 $display("U=%h B=%0d", U, $bits(U));'])
mod("Dfe8u", [ARR], ["localparam U = '1 ^ A[1];", 'initial #1 $display("U=%h B=%0d", U, $bits(U));'])
mod("Dw80v", [X8, N], ["localparam L = X + {N{40'h0}};", 'initial #1 $display("L=%h B=%0d", L, $bits(L));'])
mod("Dw80c", [X8, N], ["localparam L = (X + {N{40'h0}}) == 80'hFC;", 'initial #1 $display("L=%0d", L);'])
mod("Dw80t", [X8, N], ["localparam logic [79:0] L = X + {N{40'h0}};", 'initial #1 $display("L=%h", L);'])
mod("Dw80r", [N], ["localparam logic [79:0] L = {N{40'hFF_FFFF_FFFF}} >> 4;", 'initial #1 $display("L=%h", L);'])
mod("Dw80s", [N], ["localparam logic [79:0] L = ~{N{40'h0}};", 'initial #1 $display("L=%h", L);'])
# E. override channel (untyped / typed child parameter), and the kept-loud guard
SUB_U = "module m #(parameter P = 1);\n  initial #1 $display(\"P=%0d B=%0d\", P, $bits(P));\nendmodule\n"
SUB_T = "module m #(parameter logic [15:0] P = 0);\n  initial #1 $display(\"P=%0d\", P);\nendmodule\n"
for sn, sub in (('u', SUB_U), ('t', SUB_T)):
    for en, E, ex in (('R', "X + {N{1'b0}}", [N]), ('E', "X | A[1]", [ARR]), ('L', "X + 2'b00", []), ('Rc', "(X + {N{1'b0}}) == 8'hFC", [N])):
        mod(f"Eov{sn}_{en}", [X8] + ex, [f"m #(.P({E})) u();"], extra=sub)
SUB_K = "module m #(parameter P = -(|{2{1'b1}}));\n  initial #1 $display(\"P=%0d B=%0d\", P, $bits(P));\nendmodule\n"
SUB_K2 = "module m #(parameter int N = 2, parameter P = -(|{N{1'b1}}));\n  initial #1 $display(\"P=%0d B=%0d\", P, $bits(P));\nendmodule\n"
mod("Ekl_L", [], ["m #(.P(-5)) u();"], extra=SUB_K)
mod("Ekl_R", [], ["m #(.P(-5)) u();"], extra=SUB_K2)
mod("Ekl_Rd", [], ["m u();"], extra=SUB_K2)
mod("Ekl_Ld", [], ["m u();"], extra=SUB_K)
# F. element specifics
E1 = {
 'bit': ("localparam bit AB [0:1] = '{1'b1, 1'b0};", "X + AB[0]", "8'hFD"),
 'intu':("localparam int AI [0:1] = '{-4, 2};", "AI[0] + 32'd0", "32'hFFFF_FFFC"),
 'ints':("localparam int AI [0:1] = '{-4, 2};", "AI[0] + 0", "-4"),
 'can4':("localparam logic [3:0] A4 [0:1] = '{8'hFF, 4'h1};", "A4[0] + 4'd1", "4'd0"),
 'cans':("localparam logic signed [3:0] AS4 [0:1] = '{4'hF, 4'h1};", "AS4[0] + 4'd0", "4'hF"),
 'canss':("localparam logic signed [3:0] AS4 [0:1] = '{4'hF, 4'h1};", "AS4[0] + 4'sd0", "-4'sd1"),
 'mpk': ("localparam logic [1:0][3:0] AM [0:1] = '{8'hFC, 8'h02};", "X | AM[1]", "8'hFE"),
 'desc':("localparam logic [0:7] AD [0:1] = '{8'hFC, 8'h02};", "X | AD[1]", "8'hFE"),
 'lsb1':("localparam logic [8:1] AL [0:1] = '{8'hFC, 8'h02};", "X | AL[1]", "8'hFE"),
 'imp': ("import p::*;", "X | PA[1]", "8'hFE"),
 'pexp':("localparam logic signed [7:0] AS [0:1] = '{-8'sd4, 8'sd2};", "2 ** AS[0]", "0"),
 'u32': ("localparam logic [31:0] AW [0:1] = '{32'hFFFF_FFF0, 32'h2};", "(AW[0] + AW[1]) >> 1", "32'h7FFF_FFF9"),
 's33': ("localparam logic signed [32:0] AX [0:1] = '{-33'sd4, 33'sd2};", "AX[0] + 33'd0", "33'h1_FFFF_FFFC"),
 's64': ("localparam longint AQ [0:1] = '{-64'sd4, 64'sd2};", "(AQ[0] + 64'd0) > 64'd100", "1'b1"),
}
for en, (d, E, K) in E1.items():
    pk = en == 'imp'
    for c in ('lp', 'gi', 'rb', 'rt', 'lv'):
        cd, disp = cons(E, K, "8'd0")[c]
        mod(f"F{en}_{c}", [X8] + ([d] if not pk else []), ([d] if pk else []) + cd + ([disp] if disp else []), pkg=pk)
# generate-scope scalar shadowing the array name
mod("Fshd_lp", [X8, ARR], ["if (1) begin : g", "  localparam int A = 5;", "  localparam L = ((X | A[1]) == 8'hFC);",
    "  initial #1 $display(\"L=%0d\", L);", "end"])
# header array parameter, default and override
SUB_A = ("module m #(parameter logic [7:0] A [0:1] = '{8'hFC, 8'h02});\n  localparam logic signed [7:0] X = -4;\n"
         "  localparam L = ((X | A[1]) == 8'hFF);\n  initial #1 $display(\"L=%0d\", L);\nendmodule\n")
mod("Fhdr_d", [], ["m u();"], extra=SUB_A)
mod("Fhdr_o", [], ["m #(.A('{8'h01, 8'h03})) u();"], extra=SUB_A)
# G. element and replication inside a constant function body
FN = ("  function automatic int g{n}(input int i); return {body}; endfunction\n")
for gn, body, ex in (('Gli', "((X | A[1]) == 8'hFE)", [ARR]), ('Gvi', "((X | A[i]) == 8'hFE)", [ARR]),
                     ('Grl', "((X + {N{1'b0}}) == 8'hFC)", [N]), ('Gri', "((X + {i{1'b0}}) == 8'hFC)", [])):
    mod(f"{gn}_lp", [X8] + ex, [FN.format(n="", body=body).strip(), "localparam L = g(1);", 'initial #1 $display("L=%0d", L);'])
for n, s in cells.items():
    open(os.path.join(out, n + ".sv"), "w").write(s)
print(len(cells))
