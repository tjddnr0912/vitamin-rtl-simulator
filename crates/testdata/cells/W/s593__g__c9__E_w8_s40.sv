`timescale 1ns/1ns
module sub;
  localparam logic signed [39:0] X = -40'sd4;
  localparam R = (X ==? 8'b1111_1?00);
  initial $display("R=%0d", R);
endmodule
module top;
  sub u();
  initial #100 $finish;
endmodule
