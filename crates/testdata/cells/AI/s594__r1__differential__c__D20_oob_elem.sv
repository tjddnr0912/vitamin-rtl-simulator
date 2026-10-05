module t;
  localparam N = 4;
  localparam logic [7:0] A [2] = '{8'hFC, 8'h03};
  localparam int IA [2] = '{-4, 3};
  localparam L1 = A[3] + {N{1'b0}};
  localparam L2 = IA[2] + {N{1'b0}};
  initial $display("R: L1=%b L2=%0d B2=%0d", L1, L2, $bits(L2));
  initial #10 $finish;
endmodule
