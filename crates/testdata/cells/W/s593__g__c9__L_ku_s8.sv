`timescale 1ns/1ns
module sub #(parameter type T = logic [7:0]) ();
  localparam T X = -8'sd4;
  case (1'b1) (X ==? 4'b1?00): begin : c1 initial $display("GC=item"); end default: begin : c2 initial $display("GC=def"); end endcase
endmodule
module top;
  sub #(.T(logic signed [7:0])) u();
  initial #100 $finish;
endmodule
