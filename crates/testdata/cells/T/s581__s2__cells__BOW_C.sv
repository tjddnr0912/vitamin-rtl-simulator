`timescale 1ns/1ns
module m #(parameter logic [64:0] P = 0) (); case (1) P: begin : g initial #1 $display("BOW_P item"); end default: begin : h initial #1 $display("BOW_P default"); end endcase endmodule
module t;
  m #(.P({64'd0, (4'b1100 == 4'b1100)})) u();
endmodule
