#!/usr/bin/env python3
"""c6: consumers beyond the declaring lane: hierarchical read, routine body, aliases, port width, wider/narrower override."""
import os, sys
D = sys.argv[1]; os.makedirs(D, exist_ok=True)
PRE = "`timescale 1ns/1ns\n"; WD = "  initial #100 $finish;\n"
def cell(n, s): open(os.path.join(D, n + ".sv"), "w").write(s)
S8 = "logic signed [7:0]"
for ov, nm in [(f".T({S8})", "s8"), ("", "ctl")]:
    inst = f"sub #({ov}) u();" if ov else "sub u();"
    top = f"module top;\n  {inst}\n" + WD
    # hierarchical read from the parent
    cell(f"hier_{nm}", PRE + "module sub #(parameter type T = logic [7:0]) ();\n  localparam T X = -8'sd4;\nendmodule\n"
         + top + '  initial #1 $display("ux=%0d lt0=%0d", u.X, u.X < 0);\nendmodule\n')
    cell(f"hierH_{nm}", PRE + "module sub #(parameter type T = logic [7:0], parameter T X = -8'sd4) ();\nendmodule\n"
         + top + '  initial #1 $display("ux=%0d lt0=%0d", u.X, u.X < 0);\nendmodule\n')
    # routine body reading the constant
    cell(f"fn_{nm}", PRE + "module sub #(parameter type T = logic [7:0]) ();\n  localparam T X = -8'sd4;\n"
         "  function int f(); return X; endfunction\n  function bit g(); return X < 0; endfunction\n"
         '  initial $display("f=%0d g=%0d", f(), g());\nendmodule\n' + top + "endmodule\n")
    # constant function call folding X
    cell(f"cfn_{nm}", PRE + "module sub #(parameter type T = logic [7:0]) ();\n  localparam T X = -8'sd4;\n"
         "  function automatic int f(int a); return a + X; endfunction\n  localparam int R = f(0);\n"
         '  initial $display("R=%0d", R);\nendmodule\n' + top + "endmodule\n")
    # untyped alias in body / generate / header sibling; typed alias
    cell(f"alias_{nm}", PRE + "module sub #(parameter type T = logic [7:0]) ();\n  localparam T X = -8'sd4;\n  localparam Y = X;\n"
         '  initial $display("Y=%0d lt0=%0d b=%0d", Y, Y < 0, $bits(Y));\nendmodule\n' + top + "endmodule\n")
    cell(f"aliasG_{nm}", PRE + "module sub #(parameter type T = logic [7:0]) ();\n  localparam T X = -8'sd4;\n  if (1) begin : g\n    localparam Y = X;\n"
         '    initial $display("Y=%0d lt0=%0d b=%0d", Y, Y < 0, $bits(Y));\n  end\nendmodule\n' + top + "endmodule\n")
    cell(f"aliasH_{nm}", PRE + "module sub #(parameter type T = logic [7:0], parameter T X = -8'sd4, parameter Y = X) ();\n"
         '  initial $display("Y=%0d lt0=%0d b=%0d", Y, Y < 0, $bits(Y));\nendmodule\n' + top + "endmodule\n")
    cell(f"aliasT_{nm}", PRE + "module sub #(parameter type T = logic [7:0]) ();\n  localparam T X = -8'sd4;\n  localparam T Y = X + 1;\n"
         '  initial $display("Y=%0d lt0=%0d", Y, Y < 0);\nendmodule\n' + top + "endmodule\n")
    cell(f"derive_{nm}", PRE + "module sub #(parameter type T = logic [7:0]) ();\n  localparam T X = -8'sd4;\n  localparam Y = X + 1;\n  localparam Z = X * 2 + 1;\n"
         '  initial $display("Y=%0d Z=%0d", Y, Z);\nendmodule\n' + top + "endmodule\n")
    # port width
    cell(f"port_{nm}", PRE + "module sub #(parameter type T = logic [7:0]) (output logic [X+8:0] o);\n  localparam T X = -8'sd4;\n"
         '  initial $display("ob=%0d", $bits(o));\nendmodule\n' + f"module top;\n  wire [300:0] w;\n  {inst.replace('u();','u(.o(w));')}\n" + WD + "endmodule\n")
    # always-block run-time use
    cell(f"proc_{nm}", PRE + "module sub #(parameter type T = logic [7:0]) ();\n  localparam T X = -8'sd4;\n  int k;\n"
         "  initial begin k = 0; if (X < 0) k = 1; repeat (X + 6) k = k + 10; $display(\"k=%0d\", k); end\nendmodule\n" + top + "endmodule\n")
    # T'(X) cast and concatenation (sign-insensitive)
    cell(f"castcat_{nm}", PRE + "module sub #(parameter type T = logic [7:0]) ();\n  localparam T X = -8'sd4;\n"
         '  initial $display("c=%0d cat=%h hx=%h", T\'(X) < 0, {X, 4\'h0}, X);\nendmodule\n' + top + "endmodule\n")
# width change with sign
for nm, t in [("s16", "logic signed [15:0]"), ("s3", "logic signed [2:0]"), ("s2", "logic signed [1:0]")]:
    cell(f"wid_{nm}", PRE + "module sub #(parameter type T = logic [7:0]) ();\n  localparam T X = -4;\n"
         '  initial $display("X=%0d lt0=%0d b=%0d", X, X < 0, $bits(X));\nendmodule\n' + f"module top;\n  sub #(.T({t})) u();\n" + WD + "endmodule\n")
