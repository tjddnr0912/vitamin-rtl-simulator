`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] SN = -8'sd60;
  wire [7:0] r = {((SN ==? 4'b?100)+1){4'b1010}};
  initial #1 $display("r=%b", r);
  initial #5 $finish;
endmodule
