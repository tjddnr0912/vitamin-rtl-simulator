`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] SA = -8'sd4;
  localparam bit C = 1;
  logic [((C ? SA : 8'sd0) ==? 4'sb1?00)+3:0] v;
  initial #1 $display("vb=%0d", $bits(v));
  initial #5 $finish;
endmodule
