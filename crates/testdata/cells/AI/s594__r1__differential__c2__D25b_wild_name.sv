module t;
  localparam logic signed [7:0] X = -4;
  localparam N = 4;
  localparam L1 = ((X + {N{1'b0}}) ==? 8'b0000_00xx);
  localparam L2 = ((X + {N{1'b0}}) !=? 8'b1111_11xx);
  localparam L3 = ((X + {N{1'b0}}) ==? 8'b1111_1xx1);
  localparam L4 = ((X + {N{1'b0}}) ==? 8'b1111_11zz);
  localparam L5 = ((X + {N{1'b0}}) !=? 8'b0000_00xx);
  initial $display("R: L1=%0d L2=%0d L3=%0d L4=%0d L5=%0d", L1, L2, L3, L4, L5);
  initial #10 $finish;
endmodule
