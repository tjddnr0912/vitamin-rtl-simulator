`timescale 1ns/1ns
module t;
  localparam logic [63:0] B2 = 64'h1_0000_000C;
  logic [($signed(B2) ==? 4'sb1?00)+3:0] v;
  initial $display("vb=%0d", $bits(v));
  initial #5 $finish;
endmodule
