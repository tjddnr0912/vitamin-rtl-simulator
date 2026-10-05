`timescale 1ns/1ns
interface ifc #(parameter type T = logic [7:0]);
  T d;
endinterface
module top;
  ifc #(.T(logic [3:0])) i();
  initial begin i.d = '1; #1 $display("D=%h", i.d); #5 $finish; end
endmodule
