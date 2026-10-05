`timescale 1ns/1ns
module t;
  initial begin #1 case (1'b1) ((4'd15 + 4'd1) inside {5'b0?000}): $display("PC_N item"); default: $display("PC_N default"); endcase end
endmodule
