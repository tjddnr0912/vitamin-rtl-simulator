`timescale 1ns/1ns
module t;
  logic [((4'd15 + 4'd1) ==? 5'b1?000)+3:0] v;
  initial $display("vb=%0d", $bits(v));
  initial #5 $finish;
endmodule
