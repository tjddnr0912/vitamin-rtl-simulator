`timescale 1ns/1ns
module sub; reg [3:0] v; initial v = 4'd5; endmodule
module top;
  sub inside();
  reg [3:0] x; reg [3:0] m;
  initial begin #1 x = 4'd5; case (x) inside.v: m = 1; default: m = 0; endcase $display("hier m=%0d", m); $finish; end
  initial #1000 $finish;
endmodule
