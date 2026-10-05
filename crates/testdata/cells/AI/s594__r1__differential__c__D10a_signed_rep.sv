module t;
  localparam N = 4;
  localparam L1 = $signed({N{1'b1}}) + 8'sd0;
  localparam L2 = ($signed({N{1'b1}}) < 8'sd0);
  localparam L3 = $unsigned($signed({N{1'b1}})) + 8'd0;
  initial $display("R: L1=%0d B1=%0d L2=%0d L3=%0d", L1, $bits(L1), L2, L3);
  initial #10 $finish;
endmodule
