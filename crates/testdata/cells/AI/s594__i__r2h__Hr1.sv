`timescale 1ns/1ns
module t;
  localparam int N = 2;
  function automatic logic [({N{1'b1}} + 2'd1):0] f(input int a); f = 5'h1F; endfunction
  localparam L = f(0);
  initial #1 $display("L=%0d B=%0d", L, $bits(L));
  initial #50 $finish;
endmodule
