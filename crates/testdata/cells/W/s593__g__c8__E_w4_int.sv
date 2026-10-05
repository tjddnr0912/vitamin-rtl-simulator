`timescale 1ns/1ns
module sub;
  localparam int X = -4;
  localparam R = (X ==? 4'sb1?00);
  localparam RN = (X !=? 4'sb1?00);
  initial $display("R=%0d RN=%0d", R, RN);
endmodule
module top;
  sub u();
  initial #100 $finish;
endmodule
