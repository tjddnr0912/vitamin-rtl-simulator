#!/usr/bin/env python3
# gen3.py <outdir>: lanes the prototype newly reaches: element sign on the override channels, element in
# casts/ternaries/compares/counts/indices/labels/loops, negative / zero / shadowed name counts.
import os, sys
out = sys.argv[1]; os.makedirs(out, exist_ok=True)
cells = {}
X8 = "localparam logic signed [7:0] X = -4;"
ARR = "localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};"
ASD = "localparam logic signed [7:0] AS [0:1] = '{-8'sd4, 8'sd2};"
C1 = "localparam bit C = 1;"
def mod(name, decls, body, extra=""):
    s = "`timescale 1ns/1ns\n" + extra + "module t;\n"
    for d in decls + body: s += "  " + d + "\n"
    s += "  initial #20 $finish;\nendmodule\n"
    cells[name] = s
SUBU = "module m #(parameter P = 1);\n  initial #1 $display(\"P=%0d lt0=%0d B=%0d\", P, P < 0, $bits(P));\nendmodule\n"
SUBT = "module m #(parameter logic [15:0] P = 0);\n  initial #1 $display(\"P=%0d\", P);\nendmodule\n"
SUBS = "module m #(parameter int P = 0);\n  initial #1 $display(\"P=%0d\", P);\nendmodule\n"
for en, E in (('s', "AS[0]"), ('u', "A[0]"), ('se', "AS[0] + 8'sd0"), ('ue', "A[0] + 8'd0"), ('sm', "AS[0] | A[1]"), ('st', "C ? AS[0] : AS[1]")):
    d = [ASD, ARR, C1]
    mod(f"Ov_nu_{en}", d, [f"m #(.P({E})) u();"], SUBU)
    mod(f"Ov_pu_{en}", d, [f"m #({E}) u();"], SUBU)
    mod(f"Ov_nt_{en}", d, [f"m #(.P({E})) u();"], SUBT)
    mod(f"Ov_ns_{en}", d, [f"m #(.P({E})) u();"], SUBS)
    mod(f"Ov_df_{en}", d, ["m u();", f"defparam u.P = {E};"], SUBU)
    mod(f"Ov_lp_{en}", d, [f"localparam L = {E};", 'initial #1 $display("L=%0d lt0=%0d B=%0d", L, L < 0, $bits(L));'])
    mod(f"Ov_rt_{en}", d, [f'initial #1 $display("R=%0d lt0=%0d", {E}, ({E}) < 0);'])
misc = {
 'tern_mix': "(C ? AS[0] : A[1])",
 'tern_ss_lt': "((C ? AS[0] : AS[1]) < 0)",
 'cmp_ss': "(AS[0] < AS[1])",
 'cmp_su': "(AS[0] < A[1])",
 'cast_int': "int'(AS[0])",
 'cast_16': "16'(AS[0])",
 'cast_u16': "16'(A[0])",
 'sgn': "$signed(A[0])",
 'usgn': "$unsigned(AS[0])",
 'clog_s': "$clog2(AS[0])",
 'clog_u': "$clog2(A[0])",
 'pow_s': "(2 ** AS[1])",
 'shr_s': "(AS[0] >>> 1)",
 'shr_su': "((AS[0] + 8'd0) >>> 1)",
 'idx': "W[A[1]]",
 'idxs': "W[AS[1]]",
 'cnt': "{A[1]{1'b1}}",
 'cnts': "{AS[1]{1'b1}}",
 'neg': "-AS[0]",
 'negu': "-A[1]",
 'not': "~AS[0]",
 'mul': "(AS[0] * AS[1])",
 'mulw': "(AS[0] * AS[1] + 16'sd0)",
 'div': "(AS[0] / AS[1])",
 'divu': "(AS[0] / A[1])",
 'eq_w': "((AS[0] + 16'sd0) == -16'sd4)",
 'eq_wu': "((AS[0] + 16'd0) == 16'hFFFC)",
}
for mn, E in misc.items():
    d = [ASD, ARR, C1, "localparam logic [15:0] W = 16'h00F4;"]
    mod(f"M_{mn}_lp", d, [f"localparam L = {E};", 'initial #1 $display("L=%0d B=%0d", L, $bits(L));'])
    mod(f"M_{mn}_rb", d, [f"logic [({E}) + 0 : 0] v;", 'initial #1 $display("vb=%0d", $bits(v));'])
    mod(f"M_{mn}_rt", d, [f'initial #1 $display("R=%0d", {E});'])
# function body reading an element / generate-for over an element / case label / unsigned int element
mod("M_fn_s", [ASD], ["function automatic int g(input int i); return AS[0]; endfunction", "localparam L = g(0);",
    'initial #1 $display("L=%0d", L);'])
mod("M_fn_e", [ASD], ["function automatic int g(input int i); return (AS[0] + 8'd0); endfunction", "localparam L = g(0);",
    'initial #1 $display("L=%0d", L);'])
mod("M_gfr", [ASD], ["for (genvar g = 0; g < AS[1]; g = g + 1) begin : gl initial #1 $display(\"g=%0d\", g); end"])
mod("M_gfrn", [ASD], ["for (genvar g = AS[0]; g < 0; g = g + 1) begin : gl initial #1 $display(\"g=%0d\", g); end"])
mod("M_lab", [ARR], ["case (8'hFC) A[0]: begin : gk initial #1 $display(\"GC=item\"); end default: begin : gd initial #1 $display(\"GC=def\"); end endcase"])
mod("M_labs", [ASD], ["case (-8'sd4) AS[0]: begin : gk initial #1 $display(\"GC=item\"); end default: begin : gd initial #1 $display(\"GC=def\"); end endcase"])
mod("M_uint", ["localparam int unsigned AU [0:1] = '{32'hFFFF_FFFC, 2};"], ["localparam L = (AU[0] > 0);", "localparam V = AU[0] + 0;",
    'initial #1 $display("L=%0d V=%0d", L, V);'])
# P1 count lanes: negative / zero / shadowed / huge name counts
mod("K_neg", [X8, "localparam int NN = -2;"], ["localparam L = ((X + {NN{1'b0}}) == 8'hFC);", 'initial #1 $display("L=%0d", L);'])
mod("K_negu", [X8, "localparam int NN = -2;"], ["localparam L = {NN{1'b0}};", 'initial #1 $display("L=%0d B=%0d", L, $bits(L));'])
mod("K_zero", [X8, "localparam int NZ = 0;"], ["localparam L = (({NZ{1'b1}}, 2'b01} + X) == 8'hFD);".replace("({NZ{1'b1}}, 2'b01}", "({{NZ{1'b1}}, 2'b01}"), 'initial #1 $display("L=%0d", L);'])
mod("K_zlit", [X8], ["localparam L = (({{0{1'b1}}, 2'b01} + X) == 8'hFD);", 'initial #1 $display("L=%0d", L);'])
mod("K_shd", [X8, "localparam int N = 2;"], ["function automatic int g(input int i); int N; N = 5; return ((X + {N{1'b0}}) == 8'hFC); endfunction",
    "localparam L = g(0);", 'initial #1 $display("L=%0d", L);'])
mod("K_big", [X8, "localparam int NB = 32'h4000_0000;"], ["localparam L = ((X + {NB{1'b0}}) == 8'hFC);", 'initial #1 $display("L=%0d", L);'])
mod("K_xz", [X8, "localparam logic [3:0] NX = 4'b001x;"], ["localparam L = ((X + {NX{1'b0}}) == 8'hFC);", 'initial #1 $display("L=%0d", L);'])
mod("K_real", [X8, "localparam real NR = 2.0;"], ["localparam L = ((X + {NR{1'b0}}) == 8'hFC);", 'initial #1 $display("L=%0d", L);'])
mod("K_sgn", [X8, "localparam logic signed [3:0] NS = 4'sd2;"], ["localparam L = ((X + {NS{1'b0}}) == 8'hFC);", 'initial #1 $display("L=%0d", L);'])
mod("K_w65", ["localparam logic signed [64:0] X = -4;", "localparam int N = 2;"], ["localparam L = ((X + {N{1'b0}}) > 65'd100);", 'initial #1 $display("L=%0d", L);'])
mod("K_ovr", [], ["m #(.N(2)) u();"], "module m #(parameter int N = 1);\n  localparam logic signed [7:0] X = -4;\n  localparam L = ((X + {N{1'b0}}) == 8'hFC);\n  initial #1 $display(\"L=%0d\", L);\nendmodule\n")
mod("K_ovrU", [], ["m #(.N(2)) u();"], "module m #(parameter N = 1);\n  localparam logic signed [7:0] X = -4;\n  localparam L = ((X + {N{1'b0}}) == 8'hFC);\n  initial #1 $display(\"L=%0d\", L);\nendmodule\n")
mod("K_ovr3", [], ["m #(.N(3)) u();"], "module m #(parameter int N = 1);\n  localparam logic signed [7:0] X = -4;\n  localparam V = X + {N{3'b000}};\n  initial #1 $display(\"V=%0d B=%0d\", V, $bits(V));\nendmodule\n")
for n, s in cells.items():
    open(os.path.join(out, n + ".sv"), "w").write(s)
print(len(cells))
