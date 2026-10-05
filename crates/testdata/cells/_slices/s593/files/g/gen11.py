import os, sys
D = sys.argv[1]; os.makedirs(D, exist_ok=True)
PRE = "`timescale 1ns/1ns\n"; WD = "  initial #100 $finish;\n"
def cell(n, s): open(os.path.join(D, n + ".sv"), "w").write(s)
CONS = {
 "add0": "  localparam R = ((X + 8'sd0) ==? 4'b1?00);\n",
 "sub0": "  localparam R = ((X - 8'sd0) !=? 4'b1?00);\n",
 "neg":  "  localparam R = (-X ==? 8'b0000_01?0);\n",
 "sgn":  "  localparam R = ($signed(X) ==? 4'b1?00);\n",
 "sel":  "  localparam R = (X[7:0] ==? 8'b1111_1?00);\n",
 "rng":  "  localparam R = (X inside {[-5:-3]});\n",
 "lst":  "  localparam R = (X inside {-4, 1});\n",
 "bs":   "  localparam R = 0;\n  logic [(X ==? 4'sb1?00) + 3 : 0] v;\n",
 "wsg":  "  localparam R = (X ==? 8'sb1111_1?00);\n",
 "cmp":  "  localparam R = (X === -8'sd4);\n",
}
for c, body in CONS.items():
    disp = '  initial $display("R=%0d", R);\n' if c != "bs" else '  initial $display("vb=%0d", $bits(v));\n'
    cell(f"L_{c}", PRE + "module sub #(parameter type T = logic [7:0]) ();\n  localparam T X = -8'sd4;\n" + body + disp + "endmodule\n"
         + "module top;\n  sub #(.T(logic signed [7:0])) u();\n" + WD + "endmodule\n")
    cell(f"E_{c}", PRE + "module sub;\n  localparam logic signed [7:0] X = -8'sd4;\n" + body + disp + "endmodule\n" + "module top;\n  sub u();\n" + WD + "endmodule\n")
    cell(f"C_{c}", PRE + "module sub #(parameter type T = logic [7:0]) ();\n  localparam T X = -8'sd4;\n" + body + disp + "endmodule\n"
         + "module top;\n  sub u();\n" + WD + "endmodule\n")
