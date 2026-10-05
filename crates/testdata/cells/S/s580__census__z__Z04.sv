`timescale 1ns/1ns
module t;
  localparam signed [7:0] S = 8'sd84;
  localparam L = S inside {4'sb?100};
  initial begin $display("Z04 %b", L); #1 $finish; end
  initial #100 $finish;
endmodule
