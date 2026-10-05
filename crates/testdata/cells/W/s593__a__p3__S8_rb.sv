`timescale 1ns/1ns
module t;
  logic [(8'sb1111_1100 ==? 4'sb1?00)+3:0] v;
  initial $display("vb=%0d", $bits(v));
  initial #5 $finish;
endmodule
