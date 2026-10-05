`timescale 1ns/1ns
module sub;
  localparam logic signed [7:0] X = -8'sd4;
  int a [(X ==? 4'b1?00) + 3 : 0];
  initial $display("asz=%0d", $size(a));
endmodule
module top;
  sub u();
  initial #100 $finish;
endmodule
