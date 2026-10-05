`timescale 1ns/1ns
module sub #(parameter type T = logic [3:0], parameter T PV = '0);
  logic [(PV ==? 4'sb1?00)+3:0] v;
  initial #1 $display("vb=%0d", $bits(v));
endmodule
module t;
  sub #(.T(logic signed [63:0]), .PV(-64'sd4)) u();
  initial #5 $finish;
endmodule
