`timescale 1ns/1ns
module t;
  localparam logic [63:0] B2 = 64'h1_0000_000C;
  localparam L = ($unsigned(B2) == 4'd12);
  initial $display("L=%b", L);
  initial #5 $finish;
endmodule
