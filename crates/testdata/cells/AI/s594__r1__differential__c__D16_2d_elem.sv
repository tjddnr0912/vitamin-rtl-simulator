module t;
  localparam N = 4;
  localparam logic signed [7:0] A2 [2][2] = '{'{-8'sd4, 8'sd1}, '{8'sd2, 8'sd3}};
  localparam L1 = A2[0][0] + {N{1'b0}};
  localparam L2 = (A2[0][0] < 8'sd0);
  localparam L3 = A2[1][1] + 8'd0;
  initial $display("R: L1=%0d B1=%0d L2=%0d L3=%0d", L1, $bits(L1), L2, L3);
  initial #10 $finish;
endmodule
