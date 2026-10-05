import os, sys
D = sys.argv[1]; os.makedirs(D, exist_ok=True)
PRE = "`timescale 1ns/1ns\n"; WD = "  initial #100 $finish;\n"
def cell(n, s): open(os.path.join(D, n + ".sv"), "w").write(s)
WC = "(X ==? 4'b1?00)"
CONS = {
 "ku0": f"  case (1'b0) {WC}: begin : c1 initial $display(\"GC=item\"); end default: begin : c2 initial $display(\"GC=def\"); end endcase\n",
 "ksc": f"  case ({WC}) 1'b0: begin : c1 initial $display(\"GS=zero\"); end default: begin : c2 initial $display(\"GS=def\"); end endcase\n",
 "adim": f"  int a [{WC} + 3 : 0];\n  initial $display(\"asz=%0d\", $size(a));\n",
 "rep": f"  localparam [7:0] R = {{({WC} + 2){{1'b1}}}};\n  initial $display(\"R=%b\", R);\n",
 "psel": f"  logic [15:0] v = 16'hA5C3;\n  initial $display(\"ps=%b\", v[0 +: {WC} + 2]);\n",
 "gfor": f"  for (genvar i = 0; i < {WC} + 2; i++) begin : g\n    initial $display(\"gi=%0d\", i);\n  end\n",
 "pb":   f"  logic [{WC} + 3 : 0] v;\n  initial $display(\"vb=%0d\", $bits(v));\n",
 "tern": f"  localparam int R = {WC} ? 5 : 7;\n  initial $display(\"R=%0d\", R);\n",
}
for c, body in CONS.items():
    cell(f"L_{c}", PRE + "module sub #(parameter type T = logic [7:0]) ();\n  localparam T X = -8'sd4;\n" + body + "endmodule\n"
         + "module top;\n  sub #(.T(logic signed [7:0])) u();\n" + WD + "endmodule\n")
    cell(f"E_{c}", PRE + "module sub;\n  localparam logic signed [7:0] X = -8'sd4;\n" + body + "endmodule\n" + "module top;\n  sub u();\n" + WD + "endmodule\n")
    cell(f"C_{c}", PRE + "module sub #(parameter type T = logic [7:0]) ();\n  localparam T X = -8'sd4;\n" + body + "endmodule\n"
         + "module top;\n  sub u();\n" + WD + "endmodule\n")
