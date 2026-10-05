module t;
  localparam N = 4;
  localparam logic [15:0] P = 16'hABCD;
  localparam L1 = P[{N{1'b1}} : 0];
  localparam L2 = P[$bits({N{1'b1}}) + 3 -: 4];
  initial $display("R: L1=%h B1=%0d L2=%h", L1, $bits(L1), L2);
  initial #10 $finish;
endmodule
