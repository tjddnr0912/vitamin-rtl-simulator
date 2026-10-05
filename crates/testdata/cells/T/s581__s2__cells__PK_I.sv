`timescale 1ns/1ns
package pk; localparam K = (4'b1100 inside {4'b1?00}); endpackage
module t;
  localparam L = pk::K;
  initial #1 $display("PK_I %b", L);
endmodule
