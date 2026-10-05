`timescale 1ns/1ns
module m #(parameter type T = logic [7:0]);
  localparam T X = -4;
  wire [7:0] r = {((X ==? 4'sb1?00)+1){4'b1010}};
  initial #1 $display("r=%b", r);
endmodule
module t;
  m #(.T(logic signed [7:0])) u();
  initial #5 $finish;
endmodule
