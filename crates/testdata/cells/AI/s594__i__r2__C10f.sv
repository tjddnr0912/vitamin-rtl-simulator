module t;
  localparam logic signed [7:0] X = -8'sd4;
  localparam int N = 8;
  function automatic int f(input int a); f = ((X + {N{1'b0}}) == 8'hFC); endfunction
  localparam int L = f(0);
  localparam int M = 8'd0 + f(0);
  initial $display("L=%0d M=%0d", L, M);
  initial #100 $finish;
endmodule
