module t;
  localparam logic signed [7:0] X = -4;
  localparam N = 4;
  localparam L1 = {X + {N{1'b0}}};
  localparam L2 = {X, {N{1'b0}}};
  localparam L3 = {{N{1'b1}} + 4'd1, X};
  initial $display("R: L1=%0d B1=%0d L2=%0d B2=%0d L3=%0d B3=%0d", L1, $bits(L1), L2, $bits(L2), L3, $bits(L3));
  initial #10 $finish;
endmodule
