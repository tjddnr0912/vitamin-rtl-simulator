module t;
  localparam logic [7:0] A [2] = '{8'hFC, 8'h03};
  localparam logic signed [7:0] AS [2] = '{-8'sd4, 8'sd3};
  localparam N = 4;
  localparam L1 = 1'b1 ? AS[0] : {N{1'b0}};
  localparam L2 = ((1'b1 ? AS[0] : AS[1]) < 0);
  localparam L3 = ((1'b1 ? AS[0] : A[1]) < 0);
  initial $display("R: L1=%0d B1=%0d L2=%0d L3=%0d", L1, $bits(L1), L2, L3);
  initial #10 $finish;
endmodule
