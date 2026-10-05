`timescale 1ns/1ns
module t;
  localparam int N = 2;
  localparam logic [31:0] R = {({N{1'b1}} + 2'd2){4'hA}};
  initial #1 $display("R=%h", R);
  initial #20 $finish;
endmodule
