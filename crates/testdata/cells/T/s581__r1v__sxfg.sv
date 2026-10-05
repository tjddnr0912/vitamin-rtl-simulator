`timescale 1ns/1ns
module t;
  logic [((4'bx100 ==? 4'b1?00) & 1'b0) : 0] f;
  logic [((4'bx100 ==? 4'b1?00) ? 0 : 0) : 0] g;
  initial $display("Sxf bits %0d", $bits(f));
  initial $display("Sxg bits %0d", $bits(g));
endmodule
