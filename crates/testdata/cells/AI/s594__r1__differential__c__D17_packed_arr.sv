module t;
  localparam N = 4;
  localparam logic [1:0][7:0] PA = {8'hFC, 8'h03};
  localparam L1 = PA[1] + {N{1'b0}};
  localparam L2 = ($signed(PA[1]) < 0);
  localparam L3 = PA[1] + 8'sd0;
  initial $display("R: L1=%0d B1=%0d L2=%0d L3=%0d", L1, $bits(L1), L2, L3);
  initial #10 $finish;
endmodule
