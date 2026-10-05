package pk; localparam [64:0] E1 = 65'h1_0000_0000_0000_0000; endpackage
module top;
  import pk::*;
  typedef enum {E0, E1} e_t;
  leaf la[E1:0] ();
  initial #100 $finish;
endmodule
module leaf ();
  initial #3 $display("leaf %m");
endmodule
