`timescale 1ns/1ns
module m #(parameter logic [64:0] P = 0) (); case (1) P: begin : g initial #1 $display("BOW_P item"); end default: begin : h initial #1 $display("BOW_P default"); end endcase endmodule
module t;
  m #(.P({64'd0, ((4'd15 + 4'd1) inside {5'b0?000})})) u();
endmodule
