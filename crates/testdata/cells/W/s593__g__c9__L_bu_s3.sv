`timescale 1ns/1ns
module sub #(parameter type T = logic [3:0]) ();
  localparam T X = -3'sd4;
  logic [(X ==? 4'b1?00) + 3 : 0] v;
  initial $display("vb=%0d", $bits(v));
endmodule
module top;
  sub #(.T(logic signed [2:0])) u();
  initial #100 $finish;
endmodule
