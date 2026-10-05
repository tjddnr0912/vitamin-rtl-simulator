`timescale 1ns/1ns
module t;
  localparam int N = 2;
  function automatic logic [({N{1'b1}} + 2'd1):0] f(input int a); f = 5'h1F; endfunction
  localparam int K = 8'd0 + f(0);
  logic [f(0) + 1:0] v;
  initial #1 $display("K=%0d vb=%0d", K, $bits(v));
  initial #50 $finish;
endmodule
