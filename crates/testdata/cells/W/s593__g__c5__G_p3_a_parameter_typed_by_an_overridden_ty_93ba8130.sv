`timescale 1ns/1ns
module sub #(parameter type T = logic [3:0], parameter T PV = '0);
  initial $display("lt0=%0d bits=%0d", PV < 0, $bits(PV));
endmodule
module t;
  sub #(.T(logic signed [63:0]), .PV(-64'sd4)) u();
endmodule
