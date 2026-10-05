`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] A [0:1] = '{-4, 2};
  logic [(A[0] ==? 8'b1111_1?00)+3:0] v;
  initial begin $display("vb=%0d", $bits(v)); #1 $finish; end
endmodule
