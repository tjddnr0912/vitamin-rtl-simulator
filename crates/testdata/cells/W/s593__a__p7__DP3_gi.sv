`timescale 1ns/1ns
module sub;
  parameter logic signed [63:0] P = 0;
  if (P ==? 4'sb1?00) begin : gi initial #1 $display("GI=then"); end else begin : gi initial #1 $display("GI=else"); end
endmodule
module t;
  sub u();
  defparam u.P = -4;
  initial #5 $finish;
endmodule
