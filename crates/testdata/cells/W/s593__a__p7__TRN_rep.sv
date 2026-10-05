`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] SA = -8'sd4;
  localparam bit C = 1;
  wire [7:0] r = {(((C ? SA : 8'sd0) ==? 4'sb1?00)+1){4'b1010}};
  initial #1 $display("r=%b", r);
  initial #5 $finish;
endmodule
