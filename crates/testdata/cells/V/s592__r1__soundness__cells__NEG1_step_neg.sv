module top;
  localparam S = 1;
  if (1) begin : b
    for (genvar i = 0; i < 3; i = i + S) begin : g
      wire [3:0] w = i;
      initial #1 $display("@%m w=%0d", w);
    end
    localparam S = -1;
  end
  initial #5 $finish;
endmodule
