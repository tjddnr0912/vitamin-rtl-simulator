module t;
  localparam logic signed [7:0] AS [0:1] = '{-8'sd4, -8'sd4};
  function automatic int f(input int a); logic [15:0] t; t = 16'd0 + AS[0]; f = t; endfunction
  function automatic int g(input int a); g = ((16'd0 + AS[0]) == 16'd252); endfunction
  localparam int L = f(0);
  localparam int M = g(0);
  initial $display("L=%0d M=%0d", L, M);
  initial #100 $finish;
endmodule
