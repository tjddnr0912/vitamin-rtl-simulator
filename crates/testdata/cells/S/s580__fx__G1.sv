`timescale 1ns/1ns
module t;
  localparam logic [3:0] PV = 4'b1100;
  localparam L3 = (PV ==? 4'b0?00) || (PV ==? 4'b1?00);
  localparam L5 = !(PV ==? 4'b1?00);
  localparam L6 = (PV ==? 4'b1?00) ? 5 : 6;
  initial begin #1 $display("G1 %b %b %0d", L3, L5, L6); $finish; end
endmodule
