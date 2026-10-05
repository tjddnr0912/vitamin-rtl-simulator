`timescale 1ns/1ns
module sub #(parameter type T = logic [3:0]) ();
  localparam T X = -4;
  if (X ==? 4'sb1?00) begin : g initial $display("GI=then"); end else begin : h initial $display("GI=else"); end
endmodule
module top;
  sub #(.T(int)) u();
  initial #100 $finish;
endmodule
