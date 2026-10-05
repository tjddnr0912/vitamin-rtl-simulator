`timescale 1ns/1ns
module t;
  localparam logic [7:0] L = 8'((4'b1100 == 4'b1100));
  initial #1 $display("SC_C %b", L);
endmodule
