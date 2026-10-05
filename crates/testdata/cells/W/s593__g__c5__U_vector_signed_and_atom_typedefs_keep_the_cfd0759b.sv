module tb;
  typedef logic signed [7:0] s8; typedef int myint; typedef logic [7:0] u8;
  localparam s8 S = -1; localparam myint K = -5; localparam u8 U = -1; localparam S2 = S + 1;
  initial begin $display("DIGEST=%0d %0d %0d %0d %0d", S, K, U, S2, $bits(K)); #1 $finish; end
endmodule