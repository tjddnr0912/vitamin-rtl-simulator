module t;
  localparam logic signed [7:0] X = -4;
  localparam N = 4;
  localparam L1 = {N{1'b1}} << 4;
  localparam L2 = X >>> {N{1'b0}};
  localparam L3 = {N{1'b1}} ** 2;
  localparam L4 = (X + {N{1'b0}}) >>> 2;
  localparam L5 = 2 ** {N{1'b1}};
  initial $display("R: L1=%0d B1=%0d L2=%0d L3=%0d L4=%0d L5=%0d", L1, $bits(L1), L2, L3, L4, L5);
  initial #10 $finish;
endmodule
