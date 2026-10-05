`timescale 1ns/1ns
module m #(parameter type T = logic [7:0]);
  localparam T X = -4;
  localparam L = ((X >>> 1) ==? 8'b1111_11?0);
  initial $display("L=%b", L);
endmodule
module t;
  m #(.T(logic signed [7:0])) u();
  initial #5 $finish;
endmodule
