`timescale 1ns/1ns
module m #(parameter logic [64:0] P = 0) (); initial #1 $display("OT_P %h", P); endmodule
module t;
  m #(.P(((4'd15 + 4'd1) inside {5'b0?000}))) u();
endmodule
