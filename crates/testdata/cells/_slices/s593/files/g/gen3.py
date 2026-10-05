#!/usr/bin/env python3
"""c3: override-type axis, fill/negative defaults, containers, nested/alias, package, interface, class, defparam, positional, 2-state, parse-time table."""
import os, sys
D = sys.argv[1]
os.makedirs(D, exist_ok=True)
PRE = "`timescale 1ns/1ns\n"
WD = "  initial #100 $finish;\n"
DSP = '  initial $display("X=%0d lt0=%0d b=%0d shr=%0d", X, X < 0, $bits(X), X >>> 1);\n'
def cell(name, src):
    open(os.path.join(D, name + ".sv"), "w").write(src)
def hdr(name, tdef, xdef, ovr, prelude="", body=DSP):
    inst = f"sub #({ovr}) u();" if ovr else "sub u();"
    cell(name, PRE + prelude + f"module sub #(parameter type T = {tdef}, parameter T X = {xdef});\n" + body
         + "endmodule\n" + f"module top;\n  {inst}\n" + WD + "endmodule\n")
def lp(name, tdef, xdef, ovr, prelude="", body=DSP):
    inst = f"sub #({ovr}) u();" if ovr else "sub u();"
    cell(name, PRE + prelude + f"module sub #(parameter type T = {tdef}) ();\n  localparam T X = {xdef};\n" + body
         + "endmodule\n" + f"module top;\n  {inst}\n" + WD + "endmodule\n")

# --- override-type axis (header lane value -4 via override; localparam lane value from '1 fill)
OV = {
 "s4": "logic signed [3:0]", "s8": "logic signed [7:0]", "int": "int", "byte": "byte", "short": "shortint",
 "long": "longint", "integer": "integer", "bs8": "bit signed [7:0]", "s40": "logic signed [39:0]",
 "s64": "logic signed [63:0]", "s80": "logic signed [79:0]", "s100": "logic signed [99:0]",
 "u8": "logic [7:0]", "uint": "int unsigned", "ubyte": "byte unsigned", "u80": "logic [79:0]",
 "tds8": "s8_t", "pkg": "p::ps8_t", "sts": "ss_t", "stu": "su_t",
}
PREL = ("typedef logic signed [7:0] s8_t;\n"
        "package p; typedef logic signed [7:0] ps8_t; endpackage\n"
        "typedef struct packed signed { logic [3:0] a; logic [3:0] b; } ss_t;\n"
        "typedef struct packed { logic [3:0] a; logic [3:0] b; } su_t;\n")
for k, t in OV.items():
    hdr(f"ovH_{k}", "logic [3:0]", "'0", f".T({t}), .X(-4)", PREL)
    lp(f"ovLf_{k}", "logic [3:0]", "'1", f".T({t})", PREL)
    lp(f"ovLn_{k}", "logic [3:0]", "-4", f".T({t})", PREL)
    lp(f"ovL0_{k}", "logic [3:0]", "'0", f".T({t})", PREL)
# sign drop: signed default, unsigned override
for dk, dt in {"s8": "logic signed [7:0]", "int": "int", "byte": "byte"}.items():
    for k in ["u8", "uint", "ubyte", "stu"]:
        hdr(f"dropH_{dk}_{k}", dt, "'0", f".T({OV[k]}), .X(-4)", PREL)
        lp(f"dropLf_{dk}_{k}", dt, "'1", f".T({OV[k]})", PREL)
    # control: signed default no override
    hdr(f"dropH_{dk}_ctl", dt, "-4", "", PREL)
    lp(f"dropLf_{dk}_ctl", dt, "'1", "", PREL)
# controls without override (unsigned defaults)
for xd, nm in [("'1", "f"), ("-4", "n"), ("'0", "z"), ("4'sb1100", "sl")]:
    lp(f"ctlL{nm}_u4", "logic [3:0]", xd, "")
    hdr(f"ctlH{nm}_u4", "logic [3:0]", xd, "")
# 2-state axis: x in the value
X2 = '  initial $display("X=%b", X);\n'
lp("st2_bit", "logic [3:0]", "4'bx01z", ".T(bit [3:0])", body=X2)
lp("st2_bit_ctl", "bit [3:0]", "4'bx01z", "", body=X2)
lp("st2_logic", "bit [3:0]", "4'bx01z", ".T(logic [3:0])", body=X2)
lp("st2_logic_ctl", "logic [3:0]", "4'bx01z", "", body=X2)
cell("st2_twin_bit", PRE + "module sub;\n  localparam bit [3:0] X = 4'bx01z;\n" + X2 + "endmodule\nmodule top; sub u();\n" + WD + "endmodule\n")
hdr("st2H_bit", "logic [3:0]", "'0", ".T(bit [3:0]), .X(4'bx01z)", body=X2)
# nested / alias type params
NB = '  initial $display("X=%0d lt0=%0d b=%0d", X, X < 0, $bits(X));\n'
cell("nestH", PRE + "module sub #(parameter type T = logic [3:0], parameter type T2 = T, parameter T2 X = '1);\n" + NB
     + "endmodule\nmodule top;\n  sub #(.T(logic signed [3:0])) u();\n" + WD + "endmodule\n")
cell("nestL", PRE + "module sub #(parameter type T = logic [3:0]) ();\n  localparam type T2 = T;\n  localparam T2 X = '1;\n" + NB
     + "endmodule\nmodule top;\n  sub #(.T(logic signed [3:0])) u();\n" + WD + "endmodule\n")
cell("nestL_ctl", PRE + "module sub #(parameter type T = logic [3:0]) ();\n  localparam type T2 = T;\n  localparam T2 X = '1;\n" + NB
     + "endmodule\nmodule top;\n  sub u();\n" + WD + "endmodule\n")
cell("passthru", PRE + "module leaf #(parameter type T = logic [3:0]) ();\n  localparam T X = '1;\n" + NB
     + "endmodule\nmodule mid #(parameter type T = logic [3:0]) ();\n  leaf #(.T(T)) l();\nendmodule\n"
     + "module top;\n  mid #(.T(logic signed [3:0])) u();\n" + WD + "endmodule\n")
cell("typedefT", PRE + "module sub #(parameter type T = logic [3:0]) ();\n  typedef T tt;\n  localparam tt X = '1;\n" + NB
     + "endmodule\nmodule top;\n  sub #(.T(logic signed [3:0])) u();\n" + WD + "endmodule\n")
# positional override, defparam on X
cell("posH", PRE + "module sub #(parameter type T = logic [3:0], parameter T X = '0);\n" + NB
     + "endmodule\nmodule top;\n  sub #(logic signed [7:0], -8'sd4) u();\n" + WD + "endmodule\n")
cell("defpX", PRE + "module sub #(parameter type T = logic [3:0], parameter T X = '0);\n" + NB
     + "endmodule\nmodule top;\n  sub #(.T(logic signed [7:0])) u();\n  defparam u.X = -4;\n" + WD + "endmodule\n")
# interface header lane
cell("ifcH", PRE + "interface ifc #(parameter type T = logic [3:0], parameter T X = '1) ();\n" + NB
     + "endinterface\nmodule top;\n  ifc #(.T(logic signed [3:0])) i();\n" + WD + "endmodule\n")
cell("ifcL", PRE + "interface ifc #(parameter type T = logic [3:0]) ();\n  localparam T X = '1;\n" + NB
     + "endinterface\nmodule top;\n  ifc #(.T(logic signed [3:0])) i();\n" + WD + "endmodule\n")
# class type param
cell("clsL", PRE + "class C #(type T = logic [3:0]);\n  localparam T X = '1;\n  static function void show(); $display(\"X=%0d lt0=%0d\", X, X < 0); endfunction\nendclass\n"
     + "module top;\n  initial C#(logic signed [3:0])::show();\n" + WD + "endmodule\n")
# array localparam typed by T
cell("arrL", PRE + "module sub #(parameter type T = logic [3:0]) ();\n  localparam T A [0:1] = '{-1, -2};\n  initial $display(\"A0=%0d A1lt0=%0d\", A[0], A[1] < 0);\n"
     + "endmodule\nmodule top;\n  sub #(.T(logic signed [3:0])) u();\n" + WD + "endmodule\n")
cell("arrL_twin", PRE + "module sub;\n  localparam logic signed [3:0] A [0:1] = '{-1, -2};\n  initial $display(\"A0=%0d A1lt0=%0d\", A[0], A[1] < 0);\n"
     + "endmodule\nmodule top;\n  sub u();\n" + WD + "endmodule\n")
# const variable typed by T (carried container, control)
cell("constV", PRE + "module sub #(parameter type T = logic [3:0]) ();\n  const T X = '1;\n" + NB
     + "endmodule\nmodule top;\n  sub #(.T(logic signed [3:0])) u();\n" + WD + "endmodule\n")
# parse-time table hazards: a T-typed localparam in a parse-time consumer, no override and override
for ov, nm in [("", "ctl"), (".T(logic signed [7:0])", "s8")]:
    lp(f"ptW_{nm}", "logic [7:0]", "-4", ov, body='  logic [X+8:0] v;\n  initial $display("vb=%0d", $bits(v));\n')
    lp(f"ptS_{nm}", "logic [7:0]", "-4", ov, body='  typedef struct packed { logic [X+8:0] a; } s_t;\n  s_t s;\n  initial $display("sb=%0d", $bits(s));\n')
    lp(f"ptG_{nm}", "logic [7:0]", "-4", ov, body='  for (genvar i = 0; i < 3; i++) begin : g\n    localparam int K = i;\n  end\n  initial $display("gk=%0d", g[X+5].K);\n')
    lp(f"ptV_{nm}", "logic [7:0]", "300", ov, body='  logic [X:0] v;\n  initial $display("X=%0d vb=%0d", X, $bits(v));\n')
