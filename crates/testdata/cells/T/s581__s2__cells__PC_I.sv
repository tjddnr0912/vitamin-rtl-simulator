`timescale 1ns/1ns
module t;
  initial begin #1 case (1'b1) (4'b1100 inside {4'b1?00}): $display("PC_I item"); default: $display("PC_I default"); endcase end
endmodule
