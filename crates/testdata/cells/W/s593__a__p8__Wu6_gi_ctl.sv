`timescale 1ns/1ns
module m #(parameter type T = logic [7:0]);
  localparam T X = -4;
  localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};
  localparam int N = 2;
  localparam bit C = 1;
  if (X ==? 4'sb1?00) begin : gi initial #1 $display("GI=then"); end else begin : gi initial #1 $display("GI=else"); end
endmodule
module t;
  m u();
  initial #5 $finish;
endmodule
