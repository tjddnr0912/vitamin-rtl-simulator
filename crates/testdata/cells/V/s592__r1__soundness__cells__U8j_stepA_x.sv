module top;
  localparam A = 1;
  if (1) begin : b
    for (genvar i = 0; i < 3; i = i + A) begin : g
      wire [3:0] w = i;
      initial #1 $display("@%m w=%0d", w);
    end
    localparam [3:0] A = 4'bx;
  end
  initial #5 $finish;
endmodule
