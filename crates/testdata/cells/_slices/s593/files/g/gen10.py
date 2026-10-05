import os, sys
D = sys.argv[1]; os.makedirs(D, exist_ok=True)
PRE = "`timescale 1ns/1ns\n"; WD = "  initial #100 $finish;\n"
def cell(n, s): open(os.path.join(D, n + ".sv"), "w").write(s)
CONS = {
 "shl": '  localparam int R1 = 32\'d1 << X;\n  localparam int R2 = X << 1;\n  localparam int R3 = X >> 1;\n  initial $display("R1=%0d R2=%0d R3=%0d", R1, R2, R3);\n',
 "pow": '  localparam int R1 = X ** 2;\n  localparam int R2 = 2 ** (X + 6);\n  initial $display("R1=%0d R2=%0d", R1, R2);\n',
 "clog": '  localparam int R1 = $clog2(X + 6);\n  localparam int R2 = $clog2(X);\n  initial $display("R1=%0d R2=%0d", R1, R2);\n',
 "rep": '  localparam R1 = {(X + 6){1\'b1}};\n  initial $display("R1=%b", R1);\n',
 "sel": '  logic [15:0] v = 16\'hA5C3;\n  initial $display("s1=%h s2=%b", v[X+7 -: 4], v[X+5]);\n',
 "arr": '  int a [0:7] = \'{0,1,2,3,4,5,6,7};\n  initial $display("a=%0d", a[X+5]);\n',
 "mix": '  localparam R1 = (X < 8\'d5);\n  localparam R2 = (X < 5);\n  localparam int R3 = X + 8\'d0;\n  localparam int R4 = X + 0;\n  initial $display("R1=%0d R2=%0d R3=%0d R4=%0d", R1, R2, R3, R4);\n',
 "tern": '  localparam int R1 = (X < 0) ? X : -X;\n  localparam int R2 = 1 ? X : 8\'d0;\n  initial $display("R1=%0d R2=%0d", R1, R2);\n',
 "cat": '  localparam int R1 = {X};\n  localparam int R2 = X[7:4];\n  localparam int R3 = X[7];\n  initial $display("R1=%0d R2=%0d R3=%0d", R1, R2, R3);\n',
 "for": '  for (genvar i = X; i < X + 3; i++) begin : g\n    initial $display("i=%0d", i);\n  end\n',
 "red": '  localparam R1 = &X;\n  localparam R2 = -X;\n  localparam int R3 = ~X;\n  initial $display("R1=%0d R2=%0d R3=%0d", R1, R2, R3);\n',
 "real": '  localparam real R1 = X;\n  localparam real R2 = X * 0.5;\n  initial $display("R1=%0.2f R2=%0.2f", R1, R2);\n',
 "str": '  initial $display("h=%h d=%d o=%o b=%b", X, X, X, X);\n',
}
T8 = "logic signed [7:0]"
for c in CONS:
    cell(f"L_{c}", PRE + f"module sub #(parameter type T = logic [7:0]) ();\n  localparam T X = -8'sd4;\n" + CONS[c] + "endmodule\n"
         + f"module top;\n  sub #(.T({T8})) u();\n" + WD + "endmodule\n")
    cell(f"E_{c}", PRE + f"module sub;\n  localparam {T8} X = -8'sd4;\n" + CONS[c] + "endmodule\n" + "module top;\n  sub u();\n" + WD + "endmodule\n")
    cell(f"C_{c}", PRE + f"module sub #(parameter type T = logic [7:0]) ();\n  localparam T X = -8'sd4;\n" + CONS[c] + "endmodule\n"
         + "module top;\n  sub u();\n" + WD + "endmodule\n")
