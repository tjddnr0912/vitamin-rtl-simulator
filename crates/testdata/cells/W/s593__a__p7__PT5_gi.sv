`timescale 1ns/1ns
module sub #(parameter type T = logic [3:0], parameter T PV = '0);
  if (PV ==? 4'sb1?00) begin : gi initial #1 $display("GI=then"); end else begin : gi initial #1 $display("GI=else"); end
endmodule
module t;
  sub #(.T(logic signed [63:0]), .PV(-64'sd4)) u();
  initial #5 $finish;
endmodule
