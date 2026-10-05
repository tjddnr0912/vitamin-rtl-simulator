module t;
  localparam N = 4;
  localparam L1 = {2{{N{1'b1}}}} + 8'd1;
  localparam L2 = {N{{2{1'b1}}}} + 8'd1;
  localparam L3 = {N{{N{1'b1}}}} + 16'd1;
  initial $display("R: L1=%0d B1=%0d L2=%0d L3=%0d B3=%0d", L1, $bits(L1), L2, L3, $bits(L3));
  initial #10 $finish;
endmodule
