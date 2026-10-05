module t;
  localparam logic [7:0] A [2] = '{8'hFC, 8'h03};
  localparam logic signed [7:0] AS [2] = '{-8'sd4, 8'sd3};
  localparam L2 = $unsigned(AS[0]) + 16'sd0;
  localparam L3 = $signed(A[0]) + 16'sd0;
  localparam L4 = (AS[0] + 16'sd0);
  initial $display("R: L2=%0d L3=%0d L4=%0d B4=%0d", L2, L3, L4, $bits(L4));
  initial #10 $finish;
endmodule
