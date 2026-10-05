`timescale 1ns/1ns
package pk; localparam K = (4'b1100 == 4'b1100); endpackage
module t;
  localparam L = pk::K;
  initial #1 $display("PK_C %b", L);
endmodule
