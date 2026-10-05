`timescale 1ns/1ns
module t;
  localparam logic [7:0] L = 8'(((4'd15 + 4'd1) ==? 5'b1?000));
  initial #1 $display("SC_W %b", L);
endmodule
