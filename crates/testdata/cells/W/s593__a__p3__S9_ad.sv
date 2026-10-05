`timescale 1ns/1ns
module t;
  localparam int I = 12;
  logic [3:0] arr [0:(I inside {4'b1?00, 4'b0011})+2];
  initial $display("asz=%0d", $size(arr));
  initial #5 $finish;
endmodule
