module t;
  localparam logic signed [7:0] X = -4;
  localparam N = 4;
  localparam L1 = (X + {N{1'b0}}) inside {8'hFC};
  localparam L2 = ((X + {N{1'b0}}) ==? 8'b1111_11xx);
  localparam L3 = ((X + {N{1'b0}}) inside {[8'd250:8'd255]});
  initial $display("R: L1=%0d L2=%0d L3=%0d", L1, L2, L3);
  initial #10 $finish;
endmodule
