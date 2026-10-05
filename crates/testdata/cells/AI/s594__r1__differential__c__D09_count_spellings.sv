module t;
  localparam logic signed [7:0] X = -4;
  localparam N = 16;
  localparam L1 = ((({$clog2(65536){1'b0}} + 8'd255 + 8'd1) >> 1) == 128);
  localparam L2 = {$bits(X){1'b1}} + 8'd1;
  localparam L3 = ((({N-12{1'b0}} + 8'd255 + 8'd1) >> 1) == 128);
  localparam L4 = {N/4{1'b1}} + 4'd1;
  initial $display("R: L1=%0d L2=%0d B2=%0d L3=%0d L4=%0d B4=%0d", L1, L2, $bits(L2), L3, L4, $bits(L4));
  initial #10 $finish;
endmodule
