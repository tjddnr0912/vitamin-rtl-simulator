package pk; localparam [64:0] E1 = 65'h1_0000_0000_0000_0000; endpackage
import pk::*;
module top;
  typedef enum {E0, E1} e_t;
  localparam int K = E1;
  initial #1 $display("ecuw E1=%0d K=%0d", E1, K);
  initial #100 $finish;
endmodule
