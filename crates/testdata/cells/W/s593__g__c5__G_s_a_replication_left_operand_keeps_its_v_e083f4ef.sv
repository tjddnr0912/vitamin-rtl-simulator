`timescale 1ns/1ns
module t;
  localparam int N2 = 2;
  localparam R = ({N2{4'b1100}} ==? 8'b1?00_1100);
  initial $display("R=%0d", R);
endmodule
