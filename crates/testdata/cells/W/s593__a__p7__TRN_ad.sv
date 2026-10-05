`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] SA = -8'sd4;
  localparam bit C = 1;
  logic [3:0] arr [0:((C ? SA : 8'sd0) ==? 4'sb1?00)+2];
  initial #1 $display("asz=%0d", $size(arr));
  initial #5 $finish;
endmodule
