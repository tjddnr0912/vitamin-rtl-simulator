`timescale 1ns/1ns
module t;
  initial begin #1 case (1'b1) ((4'd15 + 4'd1) ==? 5'b1?000): $display("PC_W item"); default: $display("PC_W default"); endcase end
endmodule
