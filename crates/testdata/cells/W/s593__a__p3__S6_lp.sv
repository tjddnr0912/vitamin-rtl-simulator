`timescale 1ns/1ns
module t;
  localparam logic [63:0] B = 64'h8000_0000_0000_000C;
  localparam L = (B ==? 4'b1?00);
  initial $display("L=%b", L);
  initial #5 $finish;
endmodule
