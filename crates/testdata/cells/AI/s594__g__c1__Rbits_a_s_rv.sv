`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam bit C = 1;
  initial #1 $display("RV=%0d", X + {$bits(2'b00){1'b0}});
  initial #5 $finish;
endmodule
