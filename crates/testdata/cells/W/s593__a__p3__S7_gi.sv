`timescale 1ns/1ns
module t;
  if ((4'd15 + 4'd1) ==? 5'b1?000) begin : gi initial $display("GI=then"); end else begin : gi initial $display("GI=else"); end
  initial #5 $finish;
endmodule
