`timescale 1ns/1ns
module m #(parameter P = 0) (); case (1) P: begin : g initial #1 $display("BO_P item"); end default: begin : h initial #1 $display("BO_P default"); end endcase endmodule
module t;
  m #(.P(((4'd15 + 4'd1) ==? 5'b1?000))) u();
endmodule
