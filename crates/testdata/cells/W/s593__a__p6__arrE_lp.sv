`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] A [0:1] = '{-4, 2};
  localparam L = (A[0] ==? 8'b1111_1?00);
  localparam L2 = (A[0] ==? 4'sb1?00);
  initial begin $display("L=%b L2=%b a0=%0d", L, L2, A[0]); #1 $finish; end
endmodule
