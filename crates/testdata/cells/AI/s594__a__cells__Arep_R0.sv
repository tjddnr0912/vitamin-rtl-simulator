`timescale 1ns/1ns
module t;
  localparam int N = 2;
  initial #1 $display("rep=%h", {({N{1'b0}} + 2'd3){4'hF}});
  initial #40 $finish;
endmodule
