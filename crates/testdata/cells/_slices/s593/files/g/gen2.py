#!/usr/bin/env python3
"""c2: lane x consumer x override cells for §2 🆕 W. One instance per cell."""
import os, sys
D = sys.argv[1]
os.makedirs(D, exist_ok=True)
PRE = "`timescale 1ns/1ns\n"
WD = "  initial #100 $finish;\n"

# consumer bodies over a constant named X (8-bit-ish values chosen so sign matters)
CONS = {
 "d": '  initial $display("X=%0d lt0=%0d b=%0d", X, X < 0, $bits(X));\n',
 "a": '  initial $display("shr=%0d mul=%0d div=%0d mod=%0d", X >>> 1, X * 3, X / 2, X % 3);\n',
 "s": '  initial $display("sg=%0d us=%0d", $signed(X), $unsigned(X));\n',
 "e": '  logic [15:0] w16; int i32;\n  initial begin w16 = X; i32 = X; $display("w16=%h i32=%0d", w16, i32); end\n',
 "c": '  initial case (X) -4: $display("case=m4"); 12: $display("case=12"); 252: $display("case=252"); -1: $display("case=m1"); 15: $display("case=15"); default: $display("case=def"); endcase\n',
 "r": '  localparam int R1 = X;\n  localparam R2 = (X < 0);\n  localparam int R3 = X >>> 1;\n  initial $display("R1=%0d R2=%0d R3=%0d", R1, R2, R3);\n',
 "g": '  if (X < 0) begin : gt initial $display("gif=neg"); end else begin : ge initial $display("gif=nonneg"); end\n',
 "k": '  case (X) -4: begin : c1 initial $display("gcase=m4"); end 12: begin : c2 initial $display("gcase=12"); end default: begin : c3 initial $display("gcase=def"); end endcase\n',
 "w": '  logic [X+8:0] v;\n  initial $display("vb=%0d", $bits(v));\n',
 "x": '  initial $display("cmp=%0d mix=%0d", X > -5, X + 1\'b1);\n',
}

def cell(name, src):
    open(os.path.join(D, name + ".sv"), "w").write(src)

def hdr(tdef, pdef, ovr, cons):
    # header lane: parameter T X in the #() list
    return (PRE + f"module sub #(parameter type T = {tdef}, parameter T X = {pdef});\n" + CONS[cons]
            + "endmodule\n" + f"module top;\n  sub #({ovr}) u();\n" + WD + "endmodule\n")

def body_lp(tdef, xdef, ovr, cons):
    # body localparam lane
    return (PRE + f"module sub #(parameter type T = {tdef}) ();\n  localparam T X = {xdef};\n" + CONS[cons]
            + "endmodule\n" + f"module top;\n  sub #({ovr}) u();\n" + WD + "endmodule\n")

def body_par(tdef, xdef, ovr, cons):
    # non-ANSI body: parameter type T and parameter T X in the body (both overridable)
    return (PRE + f"module sub;\n  parameter type T = {tdef};\n  parameter T X = {xdef};\n" + CONS[cons]
            + "endmodule\n" + f"module top;\n  sub #({ovr}) u();\n" + WD + "endmodule\n")

def gen_lp(tdef, xdef, ovr, cons):
    # generate-block localparam lane
    body = CONS[cons].replace("\n  ", "\n    ")
    return (PRE + f"module sub #(parameter type T = {tdef}) ();\n  if (1) begin : gb\n    localparam T X = {xdef};\n  {body}  end\n"
            + "endmodule\n" + f"module top;\n  sub #({ovr}) u();\n" + WD + "endmodule\n")

S4 = "logic signed [7:0]"
# core matrix: 4 lanes x consumers, override T to signed 8, value -4
for cons in CONS:
    cell(f"H_{cons}_s8", hdr("logic [7:0]", "'0", f".T({S4}), .X(-8'sd4)", cons))
    cell(f"L_{cons}_s8", body_lp("logic [7:0]", "-8'sd4", f".T({S4})", cons))
    cell(f"B_{cons}_s8", body_par("logic [7:0]", "-8'sd4", f".T({S4})", cons))
    cell(f"G_{cons}_s8", gen_lp("logic [7:0]", "-8'sd4", f".T({S4})", cons))
    # controls: no override, default is the signed type itself
    cell(f"H_{cons}_ctl", hdr(S4, "-8'sd4", "", cons).replace("sub #() u();", "sub u();"))
    cell(f"L_{cons}_ctl", body_lp(S4, "-8'sd4", "", cons).replace("sub #() u();", "sub u();"))
    # controls: no override, unsigned default (value 252)
    cell(f"H_{cons}_uctl", hdr("logic [7:0]", "-8'sd4", "", cons).replace("sub #() u();", "sub u();"))
    cell(f"L_{cons}_uctl", body_lp("logic [7:0]", "-8'sd4", "", cons).replace("sub #() u();", "sub u();"))
    # explicit twin: the override type spelled on the declaration
    cell(f"E_{cons}_twin", (PRE + f"module sub #(parameter {S4} X = '0);\n" + CONS[cons] + "endmodule\n"
                            + "module top;\n  sub #(.X(-8'sd4)) u();\n" + WD + "endmodule\n"))
