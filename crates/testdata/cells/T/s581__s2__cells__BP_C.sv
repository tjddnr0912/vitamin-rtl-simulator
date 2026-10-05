`timescale 1ns/1ns
package pk; localparam K = (4'b1100 == 4'b1100); endpackage
module t;
  case (1) pk::K: begin : g initial #1 $display("BP_C item"); end default: begin : h initial #1 $display("BP_C default"); end endcase
endmodule
