`timescale 1ns/1ns
module t;
  localparam L = ((4'd15 + 4'd1) ==? 5'b1?000);
  initial $display("L=%b", L);
  initial #5 $finish;
endmodule
