#!/usr/bin/env python3
# a3gen.py <outdir>: 🆕 AE / 🆕 AC follow-ups: `const_unsigned_selfdet` (delay, $clog2) with a call / a local,
# const-fn comparison and reduction with the never-assigned read OUTSIDE the replication, +: width discriminator,
# untyped value with a call, override typed-target literal twins, iverilog repeat(x) control.
import os, sys
out = sys.argv[1]; os.makedirs(out, exist_ok=True)
cells = {}
X8  = "localparam logic signed [7:0] X = -4;"
N   = "localparam int N = 2;"
ARR = "localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};"
FL  = "function automatic logic [7:0] fl(input int a); logic [7:0] r; fl = r + a; endfunction"
FG  = "function automatic logic [7:0] fg(input int a); logic [7:0] r; r = 8'd0; fg = r + a; endfunction"
F2  = "function automatic int f2(input int a); return a; endfunction"
MON = 'always @(w) $display("w=%b t=%0t", w, $time);'
def mod(name, decls, body, extra="", wd=40):
    s = "`timescale 1ns/1ns\n" + extra + "module t;\n"
    for d in decls + body:
        s += "  " + d + "\n"
    s += f"  initial #{wd} $finish;\nendmodule\n"
    cells[name] = s
# 1. CA delay with a negative-before-masking sum and a call (AE via call / assigned twin)
for nm, E, ex in (('RAE', "(X + {N{1'b0}}) + fl(2)", [N, FL]), ('LAE', "(X + 2'b00) + fl(2)", [FL]), ('EAE', "(X | A[1]) + fl(2)", [ARR, FL]),
                  ('RG', "(X + {N{1'b0}}) + fg(2)", [N, FG]), ('LG', "(X + 2'b00) + fg(2)", [FG]), ('R1', "{f2(2){1'b0}} + 8'd5", [F2])):
    mod(f"Kdl_{nm}", [X8] + ex + ["wire w;", f"assign #({E}) w = 1'b1;"], [MON], wd=300)
# 2. $clog2 at module scope with a call
for nm, E, ex in (('RAE', "(X + {N{1'b0}}) + fl(2)", [N, FL]), ('LAE', "(X + 2'b00) + fl(2)", [FL]), ('RG', "(X + {N{1'b0}}) + fg(2)", [N, FG]),
                  ('R', "X + {N{1'b0}}", [N]), ('L', "X + 2'b00", [])):
    mod(f"Kcl_{nm}", [X8] + ex, [f"localparam L = $clog2({E});", 'initial #1 $display("L=%0d", L);'])
# 3. $clog2 / reduction / comparison inside a constant function, never-assigned local OUTSIDE the replication
FB = {
 'clR':  "function automatic int fk(input int a); logic [7:0] r; fk = $clog2((X + {N{1'b0}}) + r); endfunction",
 'clL':  "function automatic int fk(input int a); logic [7:0] r; fk = $clog2((X + 2'b00) + r); endfunction",
 'clG':  "function automatic int fk(input int a); logic [7:0] r; r = 8'd0; fk = $clog2((X + {N{1'b0}}) + r); endfunction",
 'rdR':  "function automatic int fk(input int a); logic [7:0] r; fk = |(r + {N{1'b0}}); endfunction",
 'rdL':  "function automatic int fk(input int a); logic [7:0] r; fk = |(r + 2'b00); endfunction",
 'rdG':  "function automatic int fk(input int a); logic [7:0] r; r = 8'd4; fk = |(r + {N{1'b0}}); endfunction",
 'cpR':  "function automatic int fk(input int a); logic [3:0] r; fk = ((X + {N{r}}) == 8'hFC); endfunction",
 'cpL':  "function automatic int fk(input int a); logic [3:0] r; fk = ((X + {2{r}}) == 8'hFC); endfunction",
 'cpG':  "function automatic int fk(input int a); logic [3:0] r; r = 4'd0; fk = ((X + {N{r}}) == 8'hFC); endfunction",
 'cfR':  "function automatic int fk(input int a); fk = ((X + {N{1'b0}}) + fl(a)) == 8'hFE; endfunction",
 'cfL':  "function automatic int fk(input int a); fk = ((X + 2'b00) + fl(a)) == 8'hFE; endfunction",
}
for nm, F in FB.items():
    ex = [N] if "N{" in F else []
    if "fl(" in F: ex.append(FL)
    mod(f"Kfn_{nm}_lp", [X8] + ex + [F], ["localparam L = fk(2);", 'initial #1 $display("L=%0d", L);'])
    mod(f"Kfn_{nm}_rt", [X8] + ex + [F, "int q;"], ['initial begin q = fk(2); #1 $display("RT=%0d", q); end'])
# 4. +: width with a discriminating vector (v[0]=0, v[1:0]=2)
for nm, E, ex in (('R0', "{N{1'b0}} + 2'd2", [N]), ('L0', "2'b00 + 2'd2", []), ('RAE', "{N{1'b0}} + fl(2)", [N, FL]),
                  ('LAE', "2'b00 + fl(2)", [FL]), ('EAE', "(A[1] - 8'd2) + fl(2)", [ARR, FL]), ('RG', "{N{1'b0}} + fg(2)", [N, FG])):
    mod(f"Kpw_{nm}", ex + ["logic [15:0] v = 16'hABCE;"], [f'initial #1 $display("pw=%0d", v[0 +: ({E})]);'])
# 5. repeat with an assigned callee (control for the AE pair) and iverilog's run-time repeat(x)
mod("Krp_RG", [N, FG, "int k;"], ["initial begin k = 0; repeat ({N{1'b0}} + fg(2)) k = k + 1; #1 $display(\"k=%0d\", k); end"])
mod("Krp_LG", [FG, "int k;"], ["initial begin k = 0; repeat (2'b00 + fg(2)) k = k + 1; #1 $display(\"k=%0d\", k); end"])
mod("Krp_rtx", ["logic [7:0] r;", "int k;"], ["initial begin k = 0; repeat (r) k = k + 1; #1 $display(\"k=%0d\", k); end"])
# 6. untyped value with a call
for nm, E, ex in (('RAE', "X + {N{1'b0}} + fl(2)", [N, FL]), ('LAE', "X + 2'b00 + fl(2)", [FL]), ('RG', "X + {N{1'b0}} + fg(2)", [N, FG]),
                  ('LG', "X + 2'b00 + fg(2)", [FG])):
    mod(f"Klv_{nm}", [X8] + ex, [f"localparam L = {E};", 'initial #1 $display("L=%0d B=%0d", L, $bits(L));'])
# 7. override typed-target literal twins (is the typed lane's value wrong for the literal twin too?)
SUBT = "module m #(parameter logic [15:0] P = 0);\n  initial #1 $display(\"P=%0d\", P);\nendmodule\n"
SUBI = "module m #(parameter int P = 0);\n  initial #1 $display(\"P=%0d\", P);\nendmodule\n"
for nm, E, ex in (('R', "X + {N{1'b0}}", [N]), ('L', "X + 2'b00", []), ('Rp', "(X + {N{1'b0}}) + 8'd0", [N])):
    mod(f"Kot_{nm}", [X8] + ex, [f"m #(.P({E})) u();"], SUBT)
    mod(f"Koi_{nm}", [X8] + ex, [f"m #(.P({E})) u();"], SUBI)
for n, s in cells.items():
    open(os.path.join(out, n + ".sv"), "w").write(s)
print(len(cells))
