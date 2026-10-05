package pk; localparam [64:0] E1 = 65'h1_0000_0000_0000_0000; endpackage
module top;
  import pk::*;
  generate
    typedef enum {E0, E1} e_t;
  endgenerate
  localparam int K = E1;
  initial #1 $display("egr E1=%0d K=%0d", E1, K);
  initial #100 $finish;
endmodule
