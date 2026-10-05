`timescale 1ns/1ns
module sub #(parameter type T = logic [3:0], parameter T PV = '0);
  logic [3:0] arr [0:(PV ==? 4'sb1?00)+2];
  initial #1 $display("asz=%0d", $size(arr));
endmodule
module t;
  sub #(.T(logic signed [63:0]), .PV(-64'sd4)) u();
  initial #5 $finish;
endmodule
