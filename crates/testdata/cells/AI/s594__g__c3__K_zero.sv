`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam int NZ = 0;
  localparam L = (({{NZ{1'b1}}, 2'b01} + X) == 8'hFD);
  initial #1 $display("L=%0d", L);
  initial #20 $finish;
endmodule
