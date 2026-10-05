module top;
  localparam A = 1;
  if (1) begin : b
    for (genvar i = A; i < 3; i = i + 1) begin : g
      wire [3:0] w = i;
      initial #1 $display("@%m w=%0d", w);
    end
    localparam real A = 2.5;
  end
  initial #5 $finish;
endmodule
