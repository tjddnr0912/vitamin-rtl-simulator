`timescale 1ns/1ns
module m #(parameter type T = logic [3:0]) ();
  localparam T TP = '1;
  initial $display("TP=%0d lt0=%0d bits=%0d", TP, TP < 0, $bits(TP));
endmodule
module t;
  m #(.T(logic signed [3:0])) u ();
endmodule
