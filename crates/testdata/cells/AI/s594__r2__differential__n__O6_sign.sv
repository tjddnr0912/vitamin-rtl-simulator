module t;
  localparam logic signed [7:0] X = -4;
  localparam int N = 2;
  function automatic int fz(input int a); fz = a; endfunction
  localparam logic signed [7:0] AS [2] = '{-8'sd4, 8'sd3};
  localparam S = AS[0];
  localparam int K = fz(0) + (S < 0);
  localparam int K2 = (S < 0);
  initial $display("R: K=%0d K2=%0d S=%0d", K, K2, S);
  initial #40 $finish;
endmodule
