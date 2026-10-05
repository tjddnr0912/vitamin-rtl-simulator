module top;
  localparam Q = 1;
  if (1) begin : gb
    localparam P = Q + 1;
    wire [P:0] w;
    initial #1 $display("@P=%0d bits=%0d", P, $bits(w));
    localparam Q = 10;
  end
  initial #5 $finish;
endmodule
