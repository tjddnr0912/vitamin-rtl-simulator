`timescale 1ns/1ns
package pk; localparam K = (4'bx100 ==? 4'b1?00); endpackage
module t;
  localparam L = pk::K;
  initial #1 $display("PK_X %b", L);
endmodule
