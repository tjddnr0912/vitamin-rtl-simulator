`timescale 1ns/1ns
module t;
  localparam logic [3:0] PV = 4'b1100; localparam L3 = PV ==? 4'b1?00;
  initial begin $display("C12q %b", L3); #1 $finish; end
  initial #100 $finish;
endmodule
