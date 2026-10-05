`timescale 1ns/1ns
module sub;
  parameter logic signed [63:0] P = 0;
  wire [7:0] r = {((P ==? 4'sb1?00)+1){4'b1010}};
  initial #1 $display("r=%b", r);
endmodule
module t;
  sub u();
  defparam u.P = -4;
  initial #5 $finish;
endmodule
