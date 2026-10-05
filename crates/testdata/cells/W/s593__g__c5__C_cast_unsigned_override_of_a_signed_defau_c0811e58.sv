`timescale 1ns/1ns
module m #(parameter type T = logic signed [7:0]) ();
  logic signed [31:0] d;
  logic [31:0] r;
  initial begin
    d = T'(8'hF0);
    r = T'(8'hF0);
    $display("bits=%0d raw=%0d d=%0d neg=%0d", $bits(T'(8'hF0)), r, d, (T'(8'hF0) - 1) < 0);
  end
  initial #10 $finish;
endmodule
module top; m #(.T(logic [7:0])) u(); endmodule
