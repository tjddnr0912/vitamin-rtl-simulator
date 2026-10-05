`timescale 1ns/1ns
module t;
  localparam logic [63:0] B = 64'h8000_0000_0000_000C;
  logic [(B ==? 4'b1?00)+3:0] v;
  initial $display("vb=%0d", $bits(v));
  initial #5 $finish;
endmodule
