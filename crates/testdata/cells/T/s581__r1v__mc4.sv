`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] SQ = -4;
  localparam A = (8'sb1111_1100 ==? 4'sb1?00);
  localparam B = (SQ ==? 4'sb1?00);
  logic [A:0] v;
  initial $display("MC4 A=%0d B=%0d bv=%0d", A, B, $bits(v));
endmodule
