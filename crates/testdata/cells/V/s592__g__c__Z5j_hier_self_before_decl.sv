module top;
  if (1) begin : gb
    initial #1 $display("@self K=%0d", gb.K);
    localparam K = 5;
  end
  initial #5 $finish;
endmodule
