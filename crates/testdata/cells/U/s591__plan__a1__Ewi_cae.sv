package pk; localparam [64:0] E1 = 65'h1_0000_0000_0000_0000; endpackage
module top;
  import pk::*;
  typedef enum {E0, E1} e_t;
  sae #(.N(E1)) u6 ();
  initial #100 $finish;
endmodule
module sae #(parameter N = 0) ();
  function automatic integer fae(input integer x); logic [7:0] t; fae = x + t; endfunction
  localparam integer W = fae(N);
  initial #3 $display("ae %m W=%0d", W);
endmodule
