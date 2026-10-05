package pk; localparam [64:0] E1 = 65'h1_0000_0000_0000_0000; endpackage
module top;
  import pk::*;
  typedef enum {E0, E1} e_t;
  saer #(.N(E1)) u9 (.a('0));
  initial #100 $finish;
endmodule
module saer #(parameter N = 0) (input logic [N:0] a);
  function automatic integer fae(input integer x); logic [7:0] t; fae = x + t; endfunction
  localparam integer W = fae(N);
  initial #3 $display("aer %m W=%0d b=%0d", W, $bits(a));
endmodule
