module top;
  wire [K-1:0] w = '1;
  initial #1 $display("@bits=%0d K=%0d", $bits(w), K);
  generate
    localparam K = 8;
  endgenerate
  initial #5 $finish;
endmodule
