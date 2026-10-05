`timescale 1ns/1ns
module m #(parameter type T = logic [7:0]);
  localparam T X = -4;
  localparam L = (X ==? 8'b1111_1?00);
  initial $display("L=%b", L);
endmodule
module t;
  m u();
  initial #5 $finish;
endmodule
