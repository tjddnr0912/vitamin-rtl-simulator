import os, sys
D = sys.argv[1]; os.makedirs(D, exist_ok=True)
PRE = "`timescale 1ns/1ns\n"; WD = "  initial #100 $finish;\n"
def cell(n, s): open(os.path.join(D, n + ".sv"), "w").write(s)
CONS = {
 "w4": '  localparam R = (X ==? 4\'sb1?00);\n  localparam RN = (X !=? 4\'sb1?00);\n  initial $display("R=%0d RN=%0d", R, RN);\n',
 "wu": '  localparam R = (X ==? 4\'b1?00);\n  initial $display("R=%0d", R);\n',
 "in": '  localparam R = (X inside {4\'sb1?00});\n  initial $display("R=%0d", R);\n',
 "gi": '  if (X ==? 4\'sb1?00) begin : g initial $display("GI=then"); end else begin : h initial $display("GI=else"); end\n',
 "rt": '  logic r; initial begin r = (X ==? 4\'sb1?00); $display("RT=%0d", r); end\n',
}
OV = {"s8": ("logic [7:0]", "logic signed [7:0]", "-8'sd4"), "int": ("logic [3:0]", "int", "-4"),
      "s64": ("logic [3:0]", "logic signed [63:0]", "-64'sd4"), "p12": ("logic [3:0]", "logic signed [7:0]", "8'sd12"),
      "u8": ("logic signed [7:0]", "logic [7:0]", "8'd252")}
for c in CONS:
    for k, (d, t, v) in OV.items():
        cell(f"H_{c}_{k}", PRE + f"module sub #(parameter type T = {d}, parameter T X = '0);\n" + CONS[c] + "endmodule\n"
             + f"module top;\n  sub #(.T({t}), .X({v})) u();\n" + WD + "endmodule\n")
        cell(f"L_{c}_{k}", PRE + f"module sub #(parameter type T = {d}) ();\n  localparam T X = {v};\n" + CONS[c] + "endmodule\n"
             + f"module top;\n  sub #(.T({t})) u();\n" + WD + "endmodule\n")
        # explicit twin of the override type
        cell(f"E_{c}_{k}", PRE + f"module sub;\n  localparam {t} X = {v};\n" + CONS[c] + "endmodule\n"
             + "module top;\n  sub u();\n" + WD + "endmodule\n")
