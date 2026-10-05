`timescale 1ns/1ns
module t;
  initial begin #1 case (1'b1) (4'bx100 ==? 4'b1?00): $display("PC_X item"); default: $display("PC_X default"); endcase end
endmodule
