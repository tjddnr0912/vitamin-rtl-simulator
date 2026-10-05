module top;
  if (1) begin : gb
    logic [7:0] v = K;
    initial #1 $display("@v=%0d", v);
    localparam K = 8;
  end
  initial #5 $finish;
endmodule
