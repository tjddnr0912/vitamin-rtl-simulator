#!/usr/bin/env python3
# a2gen.py <outdir>: element TYPE census (ER §4.5: each spelling of the element type) + base-shadow cells for P2/P3.
import os, sys
out = sys.argv[1]; os.makedirs(out, exist_ok=True)
cells = {}
def mod(name, decls, body, extra="", wd=40):
    s = "`timescale 1ns/1ns\n" + extra + "module t;\n"
    for d in decls + body:
        s += "  " + d + "\n"
    s += f"  initial #{wd} $finish;\nendmodule\n"
    cells[name] = s
GI = lambda c: f'if ({c}) begin : gt initial #1 $display("GI=then"); end else begin : ge initial #1 $display("GI=else"); end'
# (name, decls, sign-test expr (expected per IEEE), width-test expr)
types = [
 ('en4',  ["typedef enum logic [3:0] {E0 = 4'd0, E9 = 4'd9, E12 = 4'd12} e_t;", "localparam e_t TA [0:1] = '{E12, E9};"],
          "((TA[0] + 1'sb0) < 0)", "((TA[0] + 1'b0) == 4'd12)", "((TA[0] + 1'b1) == 4'd13)"),
 ('enI',  ["typedef enum {F0, F1, F2} f_t;", "localparam f_t TA [0:1] = '{F2, F1};"],
          "((TA[0] - 3) < 0)", "((TA[0] + 1'b0) == 32'd2)", "((TA[1] - 2) < 0)"),
 ('sps',  ["typedef struct packed signed { logic [3:0] hi; logic [3:0] lo; } s_t;", "localparam s_t TA [0:1] = '{8'hF0, 8'h12};"],
          "((TA[0] + 1'sb0) < 0)", "((TA[0] + 1'b0) == 8'hF0)", "((TA[0] >>> 4) == 8'hFF)"),
 ('spu',  ["typedef struct packed { logic [3:0] hi; logic [3:0] lo; } u_t;", "localparam u_t TA [0:1] = '{8'hF0, 8'h12};"],
          "((TA[0] + 1'sb0) < 0)", "((TA[0] + 1'b0) == 8'hF0)", "((TA[0] >>> 4) == 8'h0F)"),
 ('int',  ["localparam integer TA [0:1] = '{-5, 7};"],
          "((TA[0] + 1'sb0) < 0)", "((TA[0] + 1'b0) == 32'hFFFFFFFB)", "((TA[0] >>> 1) == -3)"),
 ('tim',  ["localparam time TA [0:1] = '{64'd5, 64'd7};"],
          "(((TA[0] + 1'sb0) - 7) < 0)", "((TA[0] + 1'b0) == 64'd5)", "((TA[0] - 64'd6) > 64'd100)"),
 ('byt',  ["localparam byte TA [0:1] = '{-8'sd3, 8'sd2};"],
          "((TA[0] + 1'sb0) < 0)", "((TA[0] + 1'b0) == 8'hFD)", "((TA[0] + 8'd4) == 8'd1)"),
 ('sho',  ["localparam shortint TA [0:1] = '{-16'sd3, 16'sd2};"],
          "((TA[0] + 1'sb0) < 0)", "((TA[0] + 1'b0) == 16'hFFFD)", "((TA[0] + 16'd4) == 16'd1)"),
 ('bit1', ["localparam bit TA [0:1] = '{1'b1, 1'b0};"],
          "((TA[0] + 1'sb0) < 0)", "((TA[0] + 1'b1) == 1'b0)", "((TA[0] + TA[0]) == 1'b0)"),
 ('ls1',  ["localparam logic signed TA [0:1] = '{1'b1, 1'b0};"],
          "((TA[0] + 1'sb0) < 0)", "((TA[0] + 1'b1) == 1'b0)", "((TA[0] + 2'sb00) == -2'sd1)"),
 ('ts6',  ["typedef logic signed [5:0] s6_t;", "localparam s6_t TA [0:1] = '{-6'sd3, 6'sd2};"],
          "((TA[0] + 1'sb0) < 0)", "((TA[0] + 1'b0) == 6'h3D)", "((TA[0] + 6'd4) == 6'd1)"),
 ('mp2',  ["localparam logic [1:0][3:0] TA [0:1] = '{8'hF0, 8'h12};"],
          "((TA[0] + 1'sb0) < 0)", "((TA[0] + 1'b0) == 8'hF0)", "((TA[0] + 8'h10) == 8'h00)"),
]
for tn, decls, es, ew, ex in types:
    for k, E in (('s', es), ('w', ew), ('x', ex)):
        mod(f"T{tn}_{k}_lp", decls, [f"localparam L = {E};", 'initial #1 $display("L=%0d", L);'])
        mod(f"T{tn}_{k}_rt", decls, [f'initial #1 $display("RT=%0d", {E});'])
    mod(f"T{tn}_s_gi", decls, [GI(es)])
    mod(f"T{tn}_w_rb", decls, [f"logic [{ew} + 3:0] v;", 'initial #1 $display("vb=%0d", $bits(v));'])
# real / string arrays: the element arm must not give them an integral width/sign
mod("Treal_lp", ["localparam real TA [0:1] = '{1.5, 2.5};"], ["localparam L = (TA[0] > 1.0);", 'initial #1 $display("L=%0d", L);'])
mod("Treal_ad", ["localparam real TA [0:1] = '{1.5, 2.5};"], ["localparam real R = TA[1] + 1;", 'initial #1 $display("R=%0.2f", R);'])
mod("Tstr_lp", ["localparam string TA [0:1] = '{\"ab\", \"c\"};"], ["localparam L = (TA[0] == \"ab\");", 'initial #1 $display("L=%0d", L);'])
# base shadows: a constant-function formal / local, and a generate-scope scalar, named like the module array
ARR = "localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};"
mod("Hfml", [ARR], ["function automatic logic fs(input logic [15:0] A); fs = ((A[1] - 1'b1) == 1'b1); endfunction",
                    "localparam L = fs(16'h0000);", 'initial #1 $display("L=%0d", L);'])
mod("Hloc", [ARR], ["function automatic logic fs(input int i); logic [15:0] A; A = 16'h0000; fs = ((A[1] - 1'b1) == 1'b1); endfunction",
                    "localparam L = fs(0);", 'initial #1 $display("L=%0d", L);'])
mod("Hgen", [ARR], ["if (1) begin : g localparam logic [7:0] A = 8'h0D; localparam L = ((A[1] - 1'b1) == 1'b1); initial #1 $display(\"L=%0d\", L); end"])
mod("Hfml_rt", [ARR], ["function automatic logic fs(input logic [15:0] A); fs = ((A[1] - 1'b1) == 1'b1); endfunction",
                    "logic q;", 'initial begin q = fs(16\'h0000); #1 $display("RT=%0d", q); end'])
# element with a local index inside a constant function (index folds only through the interpreter)
mod("Hidx", [ARR, "localparam logic signed [7:0] X = -4;"],
    ["function automatic logic fi(input int i); fi = ((X | A[i]) == 8'hFE); endfunction", "localparam L = fi(1);", 'initial #1 $display("L=%0d", L);'])
mod("Hidx_c", [ARR, "localparam logic signed [7:0] X = -4;", "localparam int i = 0;"],
    ["function automatic logic fi(input int i); fi = ((X | A[i]) == 8'hFE); endfunction", "localparam L = fi(1);", 'initial #1 $display("L=%0d", L);'])
for n, s in cells.items():
    open(os.path.join(out, n + ".sv"), "w").write(s)
print(len(cells))
