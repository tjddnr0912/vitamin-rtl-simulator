`timescale 1ns/1ns
module sub;
  localparam logic signed [7:0] X = -8'sd4;
  localparam R = 0;
  logic [(X ==? 4'sb1?00) + 3 : 0] v;
  initial $display("vb=%0d", $bits(v));
endmodule
module top;
  sub u();
  initial #100 $finish;
endmodule
