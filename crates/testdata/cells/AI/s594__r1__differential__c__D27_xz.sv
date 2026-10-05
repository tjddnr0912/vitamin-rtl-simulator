module t;
  localparam logic signed [7:0] X = -4;
  localparam N = 4;
  localparam L1 = 8'bx + {N{1'b0}};
  localparam L2 = X + {N{1'bz}};
  localparam L3 = ({N{1'bx}} == 4'd0);
  initial $display("R: L1=%b L2=%b L3=%b", L1, L2, L3);
  initial #10 $finish;
endmodule
