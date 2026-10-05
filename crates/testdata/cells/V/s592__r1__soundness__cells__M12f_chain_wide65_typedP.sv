module top;
  localparam Q = 1;
  if (1) begin : b
    localparam [99:0] P = Q;
    for (genvar i = P; i < 3; i = i + 1) begin : g
      wire [3:0] w = i;
      initial #1 $display("@%m w=%0d", w);
    end
    localparam [99:0] Q = 100'h1_0000_0000_0000_0002;
  end
  initial #5 $finish;
endmodule
