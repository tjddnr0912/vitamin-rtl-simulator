package pk; localparam [64:0] E1 = 65'h1_0000_0000_0000_0000; endpackage
module top;
  import pk::*;
  typedef enum {E0, E1} e_t;
  initial #1 $display("ewiq E1=%0d pkE1=%0d", E1, pk::E1);
  initial #100 $finish;
endmodule
