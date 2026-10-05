module top;
  if (1) begin : gb
    initial #1 $display("@K=%0d", K);
    localparam K = 8;
  end
  initial #5 $finish;
endmodule
