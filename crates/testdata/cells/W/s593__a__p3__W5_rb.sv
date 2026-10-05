`timescale 1ns/1ns
module m #(parameter type T = logic [7:0]);
  localparam T X = -4;
  logic [($unsigned(X) ==? 8'b1111_1?00)+3:0] v;
  initial $display("vb=%0d", $bits(v));
endmodule
module t;
  m #(.T(logic signed [7:0])) u();
  initial #5 $finish;
endmodule
