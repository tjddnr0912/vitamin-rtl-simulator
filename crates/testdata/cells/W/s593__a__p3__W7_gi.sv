`timescale 1ns/1ns
module m #(parameter type T = logic [7:0]);
  localparam T X = -4;
  if (X inside {8'b1111_1?00}) begin : gi initial $display("GI=then"); end else begin : gi initial $display("GI=else"); end
endmodule
module t;
  m #(.T(logic signed [7:0])) u();
  initial #5 $finish;
endmodule
