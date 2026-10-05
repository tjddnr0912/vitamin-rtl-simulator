module top;
  localparam A = 1;
  localparam B = 2;
  if (1) begin : b
    if (A == 1 && B == 2) begin : x initial #1 $display("@%m x"); end else begin : y initial #1 $display("@%m y"); end
    localparam A = 1;
    localparam B = 3;
  end
  initial #5 $finish;
endmodule
