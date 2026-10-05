module tb; typedef byte b_t; typedef longint l_t;
  localparam b_t P = 9'h100; localparam l_t L = 64'hFFFF_FFFF_FFFF_FFFF; localparam W = P + 1;
  initial begin $display("DIGEST=%0d %0d %0d %0d", P, $bits(P), L, W); #1 $finish; end
endmodule