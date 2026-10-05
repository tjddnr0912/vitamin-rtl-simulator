`timescale 1ns/1ns
module m #(parameter type T = logic [7:0]);
  localparam T A [0:1] = '{-4, 2};
  logic [3:0] arr [0:(A[0] ==? 8'b1111_1?00)+2];
  initial $display("asz=%0d", $size(arr));
endmodule
module t;
  m #(.T(logic signed [7:0])) u();
  initial #5 $finish;
endmodule
