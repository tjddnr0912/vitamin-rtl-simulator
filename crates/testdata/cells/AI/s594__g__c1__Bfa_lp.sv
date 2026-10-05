`timescale 1ns/1ns
module t;
  localparam int N = 2;
  function automatic logic [15:0] g(input int a); logic signed [7:0] x; logic [15:0] r; x = -4;
    r = x + {N{1'b0}}; return r; endfunction
  localparam L = g(0);
  initial #1 $display("L=%0d", L);
  initial #5 $finish;
endmodule
