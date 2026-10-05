`timescale 1ns/1ns
module sub #(parameter type T = logic [7:0]) (output logic [X+8:0] o);
  localparam T X = -8'sd4;
  initial $display("ob=%0d", $bits(o));
endmodule
module top;
  wire [300:0] w;
  sub #(.T(logic signed [7:0])) u(.o(w));
  initial #100 $finish;
endmodule
