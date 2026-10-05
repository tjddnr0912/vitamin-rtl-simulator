`timescale 1ns/1ns
module sub #(parameter type T = logic [3:0], parameter T X = '0);
  localparam R = (X inside {4'sb1?00});
  initial $display("R=%0d", R);
endmodule
module top;
  sub #(.T(logic signed [63:0]), .X(-64'sd4)) u();
  initial #100 $finish;
endmodule
