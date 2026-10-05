module t #(parameter W = 8);
  localparam logic [W-1:0] X = -4;
  for (genvar i = 0; i < 2; i++) begin : g
    localparam int K = i;
  end
  initial begin $display("gk=%0d", g[X-251].K); #1 $finish; end
endmodule
