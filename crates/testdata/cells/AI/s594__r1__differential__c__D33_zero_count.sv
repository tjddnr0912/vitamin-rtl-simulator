module t;
  localparam logic signed [7:0] X = -4;
  localparam Z = 0;
  localparam L1 = {X, {Z{1'b1}}};
  localparam L2 = {X, {0{1'b1}}};
  initial $display("R: L1=%0d B1=%0d L2=%0d B2=%0d", L1, $bits(L1), L2, $bits(L2));
  initial #10 $finish;
endmodule
