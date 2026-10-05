`timescale 1ns/1ns
module t;
  initial begin #1 case (1'b1) (4'b1100 == 4'b1100): $display("PC_C item"); default: $display("PC_C default"); endcase end
endmodule
