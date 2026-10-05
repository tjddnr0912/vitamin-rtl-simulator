`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  for (genvar g = 2; g < 3; g = g + 1) begin : gl
  localparam L = X + {g{1'b0}};
  initial #1 $display("L=%0d B=%0d", L, $bits(L));
  end
  initial #5 $finish;
endmodule
