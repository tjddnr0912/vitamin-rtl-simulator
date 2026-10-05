`timescale 1ns/1ns
module m #(parameter type T = logic [7:0]) ();
  T mv;
  task t; T tl; begin $display("tn=%s", $typename(tl)); end endtask
  initial begin $display("mn=%s", $typename(mv)); t(); end
endmodule
module top; m #(.T(bit [7:0])) u (); initial #10 $finish; endmodule
