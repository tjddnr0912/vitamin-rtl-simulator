package pk; localparam E1 = 7; endpackage
module top;
  import pk::*;
  typedef enum {E0, E1} e_t;
  logic [E1+1:0] v;
  initial #1 $display("wid b=%0d", $bits(v));
  initial #100 $finish;
endmodule
