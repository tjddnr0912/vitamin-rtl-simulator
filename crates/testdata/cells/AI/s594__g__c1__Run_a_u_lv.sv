`timescale 1ns/1ns
module t;
  localparam logic [7:0] X = 8'hFC;
  localparam bit C = 1;
  localparam NU = 2;
  localparam L = X + {NU{1'b0}};
  initial #1 $display("L=%0d B=%0d", L, $bits(L));
  initial #5 $finish;
endmodule
