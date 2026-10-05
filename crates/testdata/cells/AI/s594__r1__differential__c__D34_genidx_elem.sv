module t;
  localparam logic signed [7:0] AS [2] = '{-8'sd4, 8'sd3};
  for (genvar g = 0; g < 2; g++) begin : G
    localparam L = AS[g] + {4{1'b0}};
    initial $display("R: g=%0d L=%0d B=%0d", g, L, $bits(L));
  end
  initial #10 $finish;
endmodule
