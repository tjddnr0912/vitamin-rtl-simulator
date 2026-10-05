#!/usr/bin/env python3
import os
P = os.path.dirname(os.path.abspath(__file__)) + '/p'
PRE = """localparam logic signed [3:0] S4 = -4'sd4;
localparam logic [64:0] W65 = {1'b1, 64'd1};
module m #(parameter int N = 7) (); initial $display("@ovr %0d", N); endmodule
"""
shapes = [("4'sb1100", "8'sb1111_1?00"), ("4'b1100", "8'sb1111_1?00"), ("33'h1_0000_0000", "33'h1_????_???0"),
 ("-33'sd1", "64'shFFFF_FFFF_????_FFFF"), ("-1", "64'hF???_????_????_???F"), ("65'h1_0000_0000_0000_0000", "65'h1_????_????_????_????"),
 ("64'hFFFF_FFFF_FFFF_FFFE", "'bx1"), ("65'h1_0000_0000_0000_0001", "'b?1"), ("(64'hFFFF_FFFF_FFFF_FFFF + 64'd1)", "65'h1_????_????_????_???0"),
 ("S4", "8'sb1111_1?00"), ("W65", "'b?1"), ("(4'sd7 + 4'sd1)", "8'sb1???_1000"), ("8'bxxxx_0101", "8'b????_0101"), ("16'h00FF", "8'sb1???_????")]
for n, (l, p) in enumerate(shapes):
    c = f"({l} ==? {p})"
    sp = f"""{PRE}module top;
  localparam L = {c};
  typedef enum logic [1:0] {{EA = {c}, EB = {c} + 1}} e_t;
  localparam logic [7:0] PV = 8'hA5;
  logic arr [{c} + 1];
  localparam int LI = ({l} !=? {p});
  localparam logic [{c}:0] LB = '1;
  m #(.N({c})) u ();
  for (genvar i = 0; i <= {c}; i++) begin : gf initial $display("@gf %0d", i); end
  initial begin
    $display("@rep %0d", $bits({{({c} + 1){{1'b1}}}}));
    $display("@cast %0d", $bits(({c} + 2)'(4'hF)));
    $display("@enum %0d %0d", EA, EB);
    $display("@psel %0d %b", $bits(PV[{c} + 1 : 0]), PV[{c} + 1 : 0]);
    $display("@arr %0d", $size(arr));
    $display("@bits %0d", $bits({c}));
    $display("@LI %0d LB=%0d L=%b", LI, $bits(LB), L);
  end
endmodule
"""
    sg = f"""{PRE}module top;
  case (1) {c}: begin : a initial $display("@g1 hit"); end default: begin : ad initial $display("@g1 def"); end endcase
  case (0) {c}: begin : b initial $display("@g0 hit"); end default: begin : bd initial $display("@g0 def"); end endcase
  case ({c}) 1'b1: begin : e initial $display("@gs 1"); end 1'b0: begin : e2 initial $display("@gs 0"); end default: begin : ed initial $display("@gs def"); end endcase
endmodule
"""
    open(f'{P}/Sp{n:02d}.sv', 'w').write(sp); open(f'{P}/Sg{n:02d}.sv', 'w').write(sg)
print(len(shapes))
