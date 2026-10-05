`timescale 1ns/1ns
package pk; localparam K = ((4'd15 + 4'd1) ==? 5'b1?000); endpackage
module t;
  case (1) pk::K: begin : g initial #1 $display("BP_W item"); end default: begin : h initial #1 $display("BP_W default"); end endcase
endmodule
