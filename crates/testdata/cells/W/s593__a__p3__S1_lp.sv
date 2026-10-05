`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] SN = -8'sd60;
  localparam L = (SN ==? 4'b?100);
  initial $display("L=%b", L);
  initial #5 $finish;
endmodule
