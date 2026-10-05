package pk;
  localparam [64:0] E1 = 65'h1_0000_0000_0000_0000;
endpackage
module top;
  import pk::*;
  typedef enum {E0, E1} e_t;
  localparam int K = E1;
  initial $display("B2D E1=%0d K=%0d", E1, K);
  initial #100 $finish;
endmodule
