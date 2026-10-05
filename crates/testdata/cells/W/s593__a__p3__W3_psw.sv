`timescale 1ns/1ns
module m #(parameter type T = logic [7:0]);
  localparam T A [0:1] = '{-4, 2};
  wire [7:0] s = 8'hA5;
  wire [7:0] ps = s[0 +: (A[0] ==? 8'b1111_1?00)+3];
  initial #1 $display("ps=%b", ps);
endmodule
module t;
  m #(.T(logic signed [7:0])) u();
  initial #5 $finish;
endmodule
