`timescale 1ns/1ns
module t;
  initial #1 $display("rep=%h", {(2'b00 + 2'd3){4'hF}});
  initial #40 $finish;
endmodule
