module top;
  localparam logic signed [7:0] SQ = -4;
  localparam A = (8'sb1111_1100 ==? 4'sb1?00);
  localparam B = (SQ ==? 4'sb1?00);
  localparam C = (SQ inside {4'sb1?00});
  logic [B:0] v;
  initial $display("MC3 A=%0d B=%0d C=%0d bv=%0d", A, B, C, $bits(v));
endmodule
