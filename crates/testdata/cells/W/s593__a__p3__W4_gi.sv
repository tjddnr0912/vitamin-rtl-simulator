`timescale 1ns/1ns
module m #(parameter type T = logic [7:0]);
  if (1) begin : g localparam T Y = -4; end
  if (g.Y ==? 8'b1111_1?00) begin : gi initial $display("GI=then"); end else begin : gi initial $display("GI=else"); end
endmodule
module t;
  m #(.T(logic signed [7:0])) u();
  initial #5 $finish;
endmodule
