`timescale 1ns/1ns
module t;
  localparam logic [7:0] L = 8'((4'b1100 inside {4'b1?00}));
  initial #1 $display("SC_I %b", L);
endmodule
