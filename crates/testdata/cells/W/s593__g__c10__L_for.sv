`timescale 1ns/1ns
module sub #(parameter type T = logic [7:0]) ();
  localparam T X = -8'sd4;
  for (genvar i = X; i < X + 3; i++) begin : g
    initial $display("i=%0d", i);
  end
endmodule
module top;
  sub #(.T(logic signed [7:0])) u();
  initial #100 $finish;
endmodule
