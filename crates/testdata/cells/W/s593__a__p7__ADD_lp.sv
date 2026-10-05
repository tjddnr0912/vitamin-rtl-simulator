`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] SA = -8'sd4;
  localparam L = ((SA + 8'sd0) ==? 4'sb1?00);
  initial #1 $display("L=%b", L);
  initial #5 $finish;
endmodule
