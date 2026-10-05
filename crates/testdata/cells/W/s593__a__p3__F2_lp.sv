`timescale 1ns/1ns
module t;
  localparam logic [63:0] B2 = 64'h1_0000_000C;
  localparam L = ($signed(B2) ==? 4'sb1?00);
  initial $display("L=%b", L);
  initial #5 $finish;
endmodule
