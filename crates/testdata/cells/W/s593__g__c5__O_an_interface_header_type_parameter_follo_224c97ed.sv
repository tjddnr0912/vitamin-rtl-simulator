`timescale 1ns/1ns
interface ifc #(parameter type T = logic [7:0]) ();
  T v;
endinterface
module top;
  ifc #(.T(logic signed [7:0])) i ();
  initial begin i.v = -1; $display("iface m1=%0d neg=%0d", i.v, (i.v < 0)); #10 $finish; end
endmodule
