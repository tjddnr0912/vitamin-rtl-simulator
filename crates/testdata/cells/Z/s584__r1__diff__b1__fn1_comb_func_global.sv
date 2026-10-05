module top;
  logic [1:0] a = 2'd1;
  logic [1:0] b;
  logic [1:0] y;
  function automatic logic [1:0] f(input logic [1:0] x); return x ^ b; endfunction
  always_comb y = f(a);
  initial b = 2'd3;
  initial #1 $display("t=%0t y=%b", $time, y);
  initial #10 $finish;
endmodule
