package pk; logic [7:0] E1 = 8'd5; endpackage
module top;
  import pk::*;
  typedef enum {E0, E1} e_t;
  e_t s;
  initial begin s = E1; #1 $display("evar2 s=%0d E1=%0d", s, E1); end
  initial #100 $finish;
endmodule
