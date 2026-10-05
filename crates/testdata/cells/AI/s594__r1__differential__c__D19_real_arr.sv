module t;
  localparam N = 4;
  localparam real RA [2] = '{1.5, -2.5};
  localparam real L1 = RA[1] + 1;
  localparam L2 = RA[0] + {N{1'b0}};
  localparam L3 = (RA[1] < 0);
  initial $display("R: L1=%0.2f L2=%0.2f L3=%0d", L1, L2, L3);
  initial #10 $finish;
endmodule
