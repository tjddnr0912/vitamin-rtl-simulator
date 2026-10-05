module top;
  initial #1 $display("@K=%0d", K);
  generate
    localparam K = 5;
  endgenerate
  initial #5 $finish;
endmodule
