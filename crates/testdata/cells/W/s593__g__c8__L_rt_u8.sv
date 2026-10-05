`timescale 1ns/1ns
module sub #(parameter type T = logic signed [7:0]) ();
  localparam T X = 8'd252;
  logic r; initial begin r = (X ==? 4'sb1?00); $display("RT=%0d", r); end
endmodule
module top;
  sub #(.T(logic [7:0])) u();
  initial #100 $finish;
endmodule
