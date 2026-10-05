`timescale 1ns/1ns
module t;
  localparam logic [7:0] L = 8'(((4'd15 + 4'd1) inside {5'b0?000}));
  initial #1 $display("SC_N %b", L);
endmodule
