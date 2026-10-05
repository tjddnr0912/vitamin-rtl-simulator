package pk; localparam [64:0] E1 = 65'h1_0000_0000_0000_0000; endpackage
module top;
  import pk::*;
  typedef enum {E0, E1} e_t;
  initial #1 $display("cmp eq1=%0d lt=%0d", (E1 == 1), (E1 < 1));
  initial #100 $finish;
endmodule
