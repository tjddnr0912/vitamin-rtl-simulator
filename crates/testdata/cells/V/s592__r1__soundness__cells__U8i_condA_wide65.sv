module top;
  localparam A = 2;
  if (1) begin : b
    for (genvar i = 0; i < A; i = i + 1) begin : g
      wire [3:0] w = i;
      initial #1 $display("@%m w=%0d", w);
    end
    localparam [99:0] A = 100'h1_0000_0000_0000_0002;
  end
  initial #5 $finish;
endmodule
