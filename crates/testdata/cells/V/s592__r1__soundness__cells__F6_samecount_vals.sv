module top;
  localparam A = 1;
  if (1) begin : b
    for (genvar i = A; i < A + 2; i = i + 1) begin : g
      initial #1 $display("@%m");
    end
    localparam A = 2;
  end
  initial #5 $finish;
endmodule
