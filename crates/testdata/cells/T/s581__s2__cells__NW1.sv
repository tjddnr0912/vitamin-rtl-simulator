`timescale 1ns/1ns
module t;
  localparam L = t.u.v ==? 4'b1?00;
  initial #1 $display("NW1 %b", L);
endmodule
