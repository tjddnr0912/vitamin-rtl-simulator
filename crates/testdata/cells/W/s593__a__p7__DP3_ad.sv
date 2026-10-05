`timescale 1ns/1ns
module sub;
  parameter logic signed [63:0] P = 0;
  logic [3:0] arr [0:(P ==? 4'sb1?00)+2];
  initial #1 $display("asz=%0d", $size(arr));
endmodule
module t;
  sub u();
  defparam u.P = -4;
  initial #5 $finish;
endmodule
