module top;
  if (1) begin : ga
    initial #1 $display("@ga K=%0d", gb.K);
  end
  if (1) begin : gb
    localparam K = 5;
  end
  initial #5 $finish;
endmodule
