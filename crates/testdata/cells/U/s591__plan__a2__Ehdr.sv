package pk; localparam [64:0] E1 = 65'h1_0000_0000_0000_0000; endpackage
module top import pk::*; #(parameter int X = E1) ();
  typedef enum {E0, E1} e_t;
  initial #1 $display("ehdr X=%0d E1=%0d", X, E1);
  initial #100 $finish;
endmodule
