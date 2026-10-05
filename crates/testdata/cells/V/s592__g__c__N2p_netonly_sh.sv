module top;
  localparam K = 4;
  if (1) begin : gb
    wire [K-1:0] w = '1;
    initial #1 $display("@bits=%0d w=%0d", $bits(w), w);
    localparam K = 8;
  end
  initial #5 $finish;
endmodule
