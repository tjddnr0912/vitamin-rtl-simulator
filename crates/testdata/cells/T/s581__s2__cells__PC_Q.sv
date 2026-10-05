`timescale 1ns/1ns
module t;
  initial begin #1 case (1'b1) (4'b1100 ==? 4'b1?00): $display("PC_Q item"); default: $display("PC_Q default"); endcase end
endmodule
