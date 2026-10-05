`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] SN = -8'sd60;
  logic [3:0] arr [0:(SN ==? 4'b?100)+2];
  initial $display("asz=%0d", $size(arr));
  initial #5 $finish;
endmodule
