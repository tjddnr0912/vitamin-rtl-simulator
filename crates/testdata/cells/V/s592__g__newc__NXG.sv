module top;
  localparam integer K = 1;
  if (1) begin : g
    if (K == 2) begin : x generate initial #1 $display("@a"); endgenerate end
    else begin : x generate initial #1 $display("@b"); endgenerate end
    localparam integer K = 2;
  end
  initial #5 $finish;
endmodule
