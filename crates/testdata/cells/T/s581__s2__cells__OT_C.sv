`timescale 1ns/1ns
module m #(parameter logic [64:0] P = 0) (); initial #1 $display("OT_P %h", P); endmodule
module t;
  m #(.P((4'b1100 == 4'b1100))) u();
endmodule
