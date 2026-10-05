module t;
  localparam logic signed [7:0] X = -4;
  localparam N = 70;
  localparam L1 = X + {N{1'b0}};
  localparam L2 = {N{1'b1}} + 1'b1;
  localparam logic [7:0] L3 = (X + {N{1'b0}}) >> 2;
  initial $display("R: L1=%0d B1=%0d L2=%0d B2=%0d L3=%0d", L1, $bits(L1), L2, $bits(L2), L3);
  initial #10 $finish;
endmodule
