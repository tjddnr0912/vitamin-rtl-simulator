`timescale 1ns/1ns
module t;
  localparam logic [7:0] U = 8'h0C;
  logic [3:0] arr [0:(U ==? 'b1?00)+2];
  initial $display("asz=%0d", $size(arr));
  initial #5 $finish;
endmodule
