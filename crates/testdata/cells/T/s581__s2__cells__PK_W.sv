`timescale 1ns/1ns
package pk; localparam K = ((4'd15 + 4'd1) ==? 5'b1?000); endpackage
module t;
  localparam L = pk::K;
  initial #1 $display("PK_W %b", L);
endmodule
