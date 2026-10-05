`timescale 1ns/1ns
module sub;
  localparam int X = -4;
  logic [(X ==? 4'b1?00) + 3 : 0] v;
  initial $display("vb=%0d", $bits(v));
endmodule
module top;
  sub u();
  initial #100 $finish;
endmodule
