`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam logic [3:0] NX = 4'b001x;
  localparam L = ((X + {NX{1'b0}}) == 8'hFC);
  initial #1 $display("L=%0d", L);
  initial #20 $finish;
endmodule
