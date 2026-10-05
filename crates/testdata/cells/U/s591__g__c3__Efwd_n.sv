package pk; localparam E1 = 7; endpackage
module top;
  import pk::*;
  localparam int K = E1;
  typedef enum {E0, E1} e_t;
  initial #1 $display("fwd K=%0d E1=%0d", K, E1);
  initial #100 $finish;
endmodule
