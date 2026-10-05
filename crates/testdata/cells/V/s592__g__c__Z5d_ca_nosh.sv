module top;
  if (1) begin : gb
    wire [7:0] w;
    assign w = K;
    initial #1 $display("@w=%0d", w);
    localparam K = 8;
  end
  initial #5 $finish;
endmodule
