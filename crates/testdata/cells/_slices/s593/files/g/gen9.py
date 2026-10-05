import os, sys
D = sys.argv[1]; os.makedirs(D, exist_ok=True)
PRE = "`timescale 1ns/1ns\n"; WD = "  initial #100 $finish;\n"
def cell(n, s): open(os.path.join(D, n + ".sv"), "w").write(s)
CONS = {
 "wn": '  localparam R = (X !=? 4\'b1?00);\n  initial $display("R=%0d", R);\n',
 "iu": '  localparam R = (X inside {4\'b1?00});\n  initial $display("R=%0d", R);\n',
 "gu": '  if (X ==? 4\'b1?00) begin : g initial $display("GI=then"); end else begin : h initial $display("GI=else"); end\n',
 "ku": '  case (1\'b1) (X ==? 4\'b1?00): begin : c1 initial $display("GC=item"); end default: begin : c2 initial $display("GC=def"); end endcase\n',
 "bu": '  logic [(X ==? 4\'b1?00) + 3 : 0] v;\n  initial $display("vb=%0d", $bits(v));\n',
 "w8": '  localparam R = (X ==? 8\'b1111_1?00);\n  initial $display("R=%0d", R);\n',
}
OV = {"s8": ("logic [7:0]", "logic signed [7:0]", "-8'sd4"), "int": ("logic [3:0]", "int", "-4"),
      "s40": ("logic [3:0]", "logic signed [39:0]", "-40'sd4"), "s3": ("logic [3:0]", "logic signed [2:0]", "-3'sd4")}
for c in CONS:
    for k, (d, t, v) in OV.items():
        cell(f"L_{c}_{k}", PRE + f"module sub #(parameter type T = {d}) ();\n  localparam T X = {v};\n" + CONS[c] + "endmodule\n"
             + f"module top;\n  sub #(.T({t})) u();\n" + WD + "endmodule\n")
        cell(f"E_{c}_{k}", PRE + f"module sub;\n  localparam {t} X = {v};\n" + CONS[c] + "endmodule\n"
             + "module top;\n  sub u();\n" + WD + "endmodule\n")
    cell(f"H_{c}_s8", PRE + f"module sub #(parameter type T = logic [7:0], parameter T X = '0);\n" + CONS[c] + "endmodule\n"
         + f"module top;\n  sub #(.T(logic signed [7:0]), .X(-8'sd4)) u();\n" + WD + "endmodule\n")
