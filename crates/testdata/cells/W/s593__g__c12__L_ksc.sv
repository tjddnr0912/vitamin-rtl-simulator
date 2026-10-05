`timescale 1ns/1ns
module sub #(parameter type T = logic [7:0]) ();
  localparam T X = -8'sd4;
  case ((X ==? 4'b1?00)) 1'b0: begin : c1 initial $display("GS=zero"); end default: begin : c2 initial $display("GS=def"); end endcase
endmodule
module top;
  sub #(.T(logic signed [7:0])) u();
  initial #100 $finish;
endmodule
