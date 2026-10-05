module top;
  localparam integer K = 0;
  if (1) begin : g1
    localparam integer K = 1;
    initial $display("@g1 K=%0d", K);
  end
  if (1) begin : g2
    if (K == 0) begin : a initial $display("@g2 outer"); end
    else begin : b initial $display("@g2 leak"); end
  end
  initial #10 $finish;
endmodule
