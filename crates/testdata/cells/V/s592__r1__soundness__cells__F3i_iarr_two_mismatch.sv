module sub;
  localparam K = 4;
  if (1) begin : gb
    if (K == 4) begin : x initial #1 $display("@%m x"); end else begin : y initial #1 $display("@%m y"); end
    localparam K = 8;
  end
endmodule
module top;
  sub u[1:0]();
  initial #5 $finish;
endmodule
