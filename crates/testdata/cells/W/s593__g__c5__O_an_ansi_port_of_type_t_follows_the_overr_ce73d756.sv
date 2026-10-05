`timescale 1ns/1ns
module m #(parameter type T = logic [7:0]) (input T a, output T o);
  assign o = a;
  initial begin #1; $display("port m1=%0d neg=%0d", o, (o < 0)); end
endmodule
module top;
  logic signed [7:0] w = -1; logic signed [7:0] r;
  m #(.T(logic signed [7:0])) u (.a(w), .o(r));
  initial begin #2; $display("top r=%0d", r); #10 $finish; end
endmodule
