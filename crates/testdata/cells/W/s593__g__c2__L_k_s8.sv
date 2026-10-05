`timescale 1ns/1ns
module sub #(parameter type T = logic [7:0]) ();
  localparam T X = -8'sd4;
  case (X) -4: begin : c1 initial $display("gcase=m4"); end 12: begin : c2 initial $display("gcase=12"); end default: begin : c3 initial $display("gcase=def"); end endcase
endmodule
module top;
  sub #(.T(logic signed [7:0])) u();
  initial #100 $finish;
endmodule
