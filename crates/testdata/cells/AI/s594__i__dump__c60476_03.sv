`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] AS [0:1] = '{-8'sd4, 8'sd2};
  localparam bit C = 1;
  localparam L = ((C ? AS[0] : AS[1]) < 0);
  initial #1 $display("L=%0d", L);
  initial #50 $finish;
endmodule
