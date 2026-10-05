module top #(parameter K = 3) ();
  generate
    localparam K = 5;
  endgenerate
  initial #1 $display("@K=%0d", K);
  initial #5 $finish;
endmodule
