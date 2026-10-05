`timescale 1ns/1ns
module sub;
  localparam logic signed [7:0] X = -8'sd4;
  logic [15:0] v = 16'hA5C3;
  initial $display("ps=%b", v[0 +: (X ==? 4'b1?00) + 2]);
endmodule
module top;
  sub u();
  initial #100 $finish;
endmodule
