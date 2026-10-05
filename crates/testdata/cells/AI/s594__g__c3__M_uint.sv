`timescale 1ns/1ns
module t;
  localparam int unsigned AU [0:1] = '{32'hFFFF_FFFC, 2};
  localparam L = (AU[0] > 0);
  localparam V = AU[0] + 0;
  initial #1 $display("L=%0d V=%0d", L, V);
  initial #20 $finish;
endmodule
