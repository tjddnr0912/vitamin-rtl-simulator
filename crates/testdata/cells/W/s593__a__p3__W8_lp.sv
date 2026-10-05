`timescale 1ns/1ns
module m #(parameter type T = logic [7:0]);
  localparam T X = -4;
  localparam L = ({X, 4'b0} ==? 12'b1111_1?00_0000);
  initial $display("L=%b", L);
endmodule
module t;
  m #(.T(logic signed [7:0])) u();
  initial #5 $finish;
endmodule
