`timescale 1ns/1ns
module t;
  logic [7:0] v8 = 8'hA5;
  initial #1 $display("PS_N %b", v8[((4'd15 + 4'd1) inside {5'b0?000})*3+1:0]);
endmodule
