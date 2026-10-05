`timescale 1ns/1ns
package pk; localparam K = (4'b1100 inside {4'b1?00}); endpackage
module t;
  case (1) pk::K: begin : g initial #1 $display("BP_I item"); end default: begin : h initial #1 $display("BP_I default"); end endcase
endmodule
