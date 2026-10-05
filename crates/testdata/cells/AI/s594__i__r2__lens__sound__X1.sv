module t;
  localparam logic signed [7:0] AS [0:1] = '{-8'sd4, -8'sd4};
  localparam logic [15:0] L = 16'd0 + AS[0];
  localparam int M = ((16'd0 + AS[0]) == 16'd252);
  initial $display("L=%0d M=%0d", L, M);
  initial #100 $finish;
endmodule
